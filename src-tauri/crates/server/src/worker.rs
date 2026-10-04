//! The ingest worker: a Node process running the frontend's ingest queue
//! so ingest continues with no browser tab open (plans/web-server-mode.md,
//! Phase 4).
//!
//! The worker talks to this server like a browser tab does (`/rpc`,
//! `/events`) but with its own Bearer token. Browser tabs reach the queue
//! through `ingest_command`: the server broadcasts the command as an
//! `ingest-worker://command` event, the worker executes it and answers
//! with `worker_reply`, which completes the original RPC.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use llm_wiki_core::EventSink;
use serde_json::{json, Value};
use tokio::process::Command;
use tokio::sync::oneshot;

use crate::events::EventHub;

pub const COMMAND_EVENT: &str = "ingest-worker://command";
/// Events the worker may publish to browser tabs.
pub const PUBLISH_PREFIX: &str = "ingest://";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
/// The worker heartbeats every 10s; past this it is considered gone.
const LIVENESS_WINDOW: Duration = Duration::from_secs(30);

type Reply = Result<Value, String>;

pub struct WorkerBridge {
    events: EventHub,
    pending: Mutex<HashMap<String, oneshot::Sender<Reply>>>,
    last_seen: Mutex<Option<Instant>>,
}

impl WorkerBridge {
    pub fn new(events: EventHub) -> Self {
        Self {
            events,
            pending: Mutex::new(HashMap::new()),
            last_seen: Mutex::new(None),
        }
    }

    /// Records that the worker is alive. Called on every worker request.
    pub fn touch(&self) {
        if let Ok(mut last_seen) = self.last_seen.lock() {
            *last_seen = Some(Instant::now());
        }
    }

    pub fn is_alive(&self) -> bool {
        self.last_seen
            .lock()
            .ok()
            .and_then(|last_seen| *last_seen)
            .is_some_and(|at| at.elapsed() < LIVENESS_WINDOW)
    }

    /// Runs a queue operation in the worker and waits for its result.
    pub async fn command(&self, op: &str, args: Value) -> Reply {
        if !self.is_alive() {
            return Err("The ingest worker is not running on the server".into());
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .map_err(|_| "worker bridge poisoned")?
            .insert(id.clone(), sender);
        let _ = self
            .events
            .emit_value(COMMAND_EVENT, json!({ "id": id, "op": op, "args": args }));
        let outcome = tokio::time::timeout(COMMAND_TIMEOUT, receiver).await;
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&id);
        }
        match outcome {
            Ok(Ok(reply)) => reply,
            Ok(Err(_)) => Err("The ingest worker dropped the command".into()),
            Err(_) => Err(format!("The ingest worker did not answer `{op}` in time")),
        }
    }

    pub fn reply(&self, id: &str, reply: Reply) {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(id));
        if let Some(sender) = sender {
            let _ = sender.send(reply);
        }
    }
}

pub struct WorkerLaunch {
    pub node: PathBuf,
    pub script: PathBuf,
    pub server_url: String,
    pub token: String,
}

/// Keeps the worker running for the life of the server, restarting it with
/// backoff when it exits. The child is killed when the server stops: it is
/// spawned with `kill_on_drop`, and its stdin is a pipe it watches.
pub fn supervise(launch: WorkerLaunch) {
    tokio::spawn(async move {
        let mut backoff = Duration::from_secs(1);
        loop {
            let started = Instant::now();
            let child = Command::new(&launch.node)
                .arg(&launch.script)
                .env("LLM_WIKI_SERVER_URL", &launch.server_url)
                .env("LLM_WIKI_WORKER_TOKEN", &launch.token)
                .stdin(Stdio::piped())
                .kill_on_drop(true)
                .spawn();
            match child {
                Ok(mut child) => {
                    // `wait()` closes the child's stdin first, which the worker
                    // reads as "server gone". Hold it open while it runs.
                    let _stdin = child.stdin.take();
                    let status = child.wait().await;
                    eprintln!("[worker] exited: {status:?}");
                }
                Err(e) => eprintln!(
                    "[worker] failed to start `{} {}`: {e}",
                    launch.node.display(),
                    launch.script.display()
                ),
            }
            // A worker that ran for a while gets a quick restart; one that
            // keeps dying immediately backs off up to a minute.
            backoff = if started.elapsed() > Duration::from_secs(60) {
                Duration::from_secs(1)
            } else {
                (backoff * 2).min(Duration::from_secs(60))
            };
            tokio::time::sleep(backoff).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn command_round_trips_through_the_event_stream() {
        let events = EventHub::new();
        let bridge = Arc::new(WorkerBridge::new(events.clone()));
        let mut stream = events.subscribe();
        bridge.touch();

        let worker = {
            let bridge = Arc::clone(&bridge);
            tokio::spawn(async move {
                let message: Value = serde_json::from_str(&stream.recv().await.unwrap()).unwrap();
                assert_eq!(message["event"], COMMAND_EVENT);
                assert_eq!(message["payload"]["op"], "getQueue");
                let id = message["payload"]["id"].as_str().unwrap().to_string();
                bridge.reply(&id, Ok(json!(["task"])));
            })
        };

        assert_eq!(
            bridge.command("getQueue", json!({})).await,
            Ok(json!(["task"]))
        );
        worker.await.unwrap();
    }

    #[tokio::test]
    async fn command_fails_fast_without_a_live_worker() {
        let bridge = WorkerBridge::new(EventHub::new());
        let err = bridge.command("getQueue", json!({})).await.unwrap_err();
        assert!(err.contains("not running"), "{err}");
    }
}
