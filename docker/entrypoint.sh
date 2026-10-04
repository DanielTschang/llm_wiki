#!/bin/sh
# Container entrypoint for llm-wiki-server.
#
# Hosts such as Fly.io mount volumes owned by root, which the unprivileged
# server user cannot write. When started as root, hand the data directory
# to that user, then drop privileges before running the server.
set -e

DATA_DIR="${LLM_WIKI_DATA_DIR:-/data}"

if [ "$(id -u)" = "0" ]; then
  mkdir -p "$DATA_DIR"
  # Only the mount point itself: everything inside was created by the
  # server user already, and a recursive chown would slow every start.
  if [ "$(stat -c %U "$DATA_DIR")" != "llmwiki" ]; then
    chown llmwiki:llmwiki "$DATA_DIR"
  fi
  exec setpriv --reuid=llmwiki --regid=llmwiki --init-groups "$@"
fi

exec "$@"
