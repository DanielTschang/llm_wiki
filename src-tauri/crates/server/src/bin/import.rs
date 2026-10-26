//! Copies a Project folder (the layout the desktop app and the single-user
//! server used) into the platform: pages go to Postgres, files under `raw/`
//! and `wiki/media/` to the object store, `.llm-wiki/` state to its table.
//! Run it once per folder; running it again into the same Project only adds
//! what is missing.
//!
//! Not carried over, because the platform keeps them elsewhere or not at all:
//! the vector index (`.llm-wiki/lancedb`, rebuilt with "re-index"), page
//! backups (`page-history`, `history`), `review-inbox.json` and the other
//! files the platform no longer uses.

use std::path::{Path, PathBuf};

use clap::Parser;
use llm_wiki_storage::{
    open_services, route, Backend, Body, ProjectFiles, ProjectId, Precondition, RelPath,
    S3Config, StoreError, WriteCtx,
};
use uuid::Uuid;

/// Larger files are skipped rather than held in memory.
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Parser)]
#[command(name = "llm-wiki-import", version)]
struct Args {
    /// The Project folder to copy.
    #[arg(long)]
    dir: PathBuf,

    /// Name of the new Project. Defaults to the folder's name.
    #[arg(long)]
    name: Option<String>,

    /// Add to an existing Project (its id) instead of creating one.
    #[arg(long)]
    into: Option<Uuid>,

    /// The Platform Admin who becomes the Project's Owner, and who the
    /// imported files are credited to.
    #[arg(long, default_value = "admin")]
    admin_user: String,

    #[arg(long, env = "LLM_WIKI_DATABASE_URL", hide_env_values = true)]
    database_url: String,
    #[arg(long, env = "LLM_WIKI_S3_ENDPOINT")]
    s3_endpoint: String,
    #[arg(long, env = "LLM_WIKI_S3_BUCKET")]
    s3_bucket: String,
    #[arg(long, env = "LLM_WIKI_S3_ACCESS_KEY")]
    s3_access_key: String,
    #[arg(long, env = "LLM_WIKI_S3_SECRET_KEY", hide_env_values = true)]
    s3_secret_key: String,
    #[arg(long, env = "LLM_WIKI_S3_REGION", default_value = "us-east-1")]
    s3_region: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Report {
    copied: usize,
    already_there: usize,
    /// (path, why)
    skipped: Vec<(String, String)>,
}

/// Every file under `root` as (path relative to it, absolute path), in path
/// order. Symbolic links are not followed.
fn walk(root: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
    fn visit(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let kind = entry.file_type()?;
            if kind.is_dir() {
                visit(&entry.path(), &rel, out)?;
            } else if kind.is_file() {
                out.push((rel, entry.path()));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    visit(root, "", &mut out)?;
    Ok(out)
}

async fn import_dir(
    files: &dyn ProjectFiles,
    project: ProjectId,
    dir: &Path,
    ctx: &WriteCtx,
) -> Result<Report, String> {
    let mut report = Report::default();
    for (rel, absolute) in walk(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let skip = |report: &mut Report, why: String| report.skipped.push((rel.clone(), why));
        if rel.ends_with(".DS_Store") || rel.starts_with(".git/") {
            continue;
        }
        let path = match RelPath::parse(&rel) {
            Ok(path) => path,
            Err(error) => {
                skip(&mut report, error.to_string());
                continue;
            }
        };
        let size = std::fs::metadata(&absolute).map(|m| m.len()).unwrap_or(0);
        if size > MAX_FILE_BYTES {
            skip(&mut report, format!("{size} bytes is over the {MAX_FILE_BYTES} limit"));
            continue;
        }
        let bytes = match std::fs::read(&absolute) {
            Ok(bytes) => bytes,
            Err(error) => {
                skip(&mut report, error.to_string());
                continue;
            }
        };
        // Text goes in as text; everything the object store keeps as bytes.
        let body = match route(&path) {
            Backend::Blobs => Body::Bytes(bytes),
            Backend::Pages | Backend::State => match String::from_utf8(bytes) {
                Ok(text) => Body::Text(text),
                Err(_) => {
                    skip(&mut report, "not valid UTF-8 text".into());
                    continue;
                }
            },
        };
        match files
            .write(project, &path, body, Precondition::CreateOnly, ctx)
            .await
        {
            Ok(_) => report.copied += 1,
            Err(StoreError::AlreadyExists(_)) => report.already_there += 1,
            Err(error) => skip(&mut report, error.to_string()),
        }
    }
    Ok(report)
}

async fn run(args: Args) -> Result<(), String> {
    let dir = args
        .dir
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", args.dir.display()))?;
    let s3 = S3Config {
        endpoint: args.s3_endpoint,
        bucket: args.s3_bucket,
        access_key: args.s3_access_key,
        secret_key: args.s3_secret_key,
        region: args.s3_region,
    };
    let services = open_services(&args.database_url, &s3, 4)
        .await
        .map_err(|e| format!("cannot open the platform: {e}"))?;
    let admin = services
        .accounts
        .find_by_username(&args.admin_user)
        .await
        .map_err(|e| e.to_string())?
        .filter(|user| user.is_platform_admin)
        .ok_or_else(|| format!("{} is not a Platform Admin", args.admin_user))?;
    let project = match args.into {
        Some(id) => ProjectId(id),
        None => {
            let name = args.name.clone().unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Imported project".into())
            });
            let created = services
                .projects
                .create(&name, &admin)
                .await
                .map_err(|e| format!("cannot create the project: {e}"))?;
            println!("created project {} ({})", created.name, created.id.0);
            created.id
        }
    };
    let mut ctx = WriteCtx::new("import");
    ctx.actor = Some(admin.id);
    let report = import_dir(services.files.as_ref(), project, &dir, &ctx).await?;
    println!(
        "{} copied, {} already there, {} skipped",
        report.copied,
        report.already_there,
        report.skipped.len()
    );
    // The old vector index is hundreds of files; say so once.
    let (index, other): (Vec<_>, Vec<_>) = report
        .skipped
        .iter()
        .partition(|(path, _)| path.starts_with(".llm-wiki/lancedb/"));
    if !index.is_empty() {
        println!("  skipped {} files of the old vector index (re-index instead)", index.len());
    }
    for (path, why) in other {
        println!("  skipped {path}: {why}");
    }
    println!("project id: {}", project.0);
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(message) = run(Args::parse()).await {
        eprintln!("llm-wiki-import: {message}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use llm_wiki_storage::MemoryStore;

    use super::*;

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[tokio::test]
    async fn a_folder_is_copied_and_a_second_run_adds_nothing() {
        let dir = tempfile::tempdir().unwrap();
        put(dir.path(), "wiki/entities/a.md", b"# A");
        put(dir.path(), "purpose.md", b"why");
        put(dir.path(), "raw/sources/paper.pdf", &[0, 159, 146, 150]);
        put(dir.path(), "wiki/media/fig.png", &[1, 2, 3]);
        put(dir.path(), ".llm-wiki/review.json", b"{}");
        put(dir.path(), ".DS_Store", b"junk");
        put(dir.path(), "wiki/broken.md", &[0xff, 0xfe, 0xfd]);

        let store = MemoryStore::new();
        let project = ProjectId(Uuid::new_v4());
        let ctx = WriteCtx::new("import");
        let first = import_dir(&store, project, dir.path(), &ctx).await.unwrap();
        assert_eq!(first.copied, 5);
        assert_eq!(first.skipped.len(), 1);
        assert_eq!(first.skipped[0].0, "wiki/broken.md");

        let read = |p: &str| {
            let rel = RelPath::parse(p).unwrap();
            let store = &store;
            async move { store.read(project, &rel).await.unwrap().0 }
        };
        assert_eq!(read("wiki/entities/a.md").await, Body::Text("# A".into()));
        assert_eq!(read("raw/sources/paper.pdf").await, Body::Bytes(vec![0, 159, 146, 150]));

        let again = import_dir(&store, project, dir.path(), &ctx).await.unwrap();
        assert_eq!((again.copied, again.already_there), (0, 5));
    }
}
