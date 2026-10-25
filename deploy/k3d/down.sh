#!/usr/bin/env bash
# Delete the LLM Wiki k3d dev cluster and all its data.
set -euo pipefail
k3d cluster delete llm-wiki
