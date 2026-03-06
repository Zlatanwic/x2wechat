#!/usr/bin/env bash
set -euo pipefail

REPO_NAME="x2wechat"
TARGET_ROOT="${HOME}/.cargo-target"
TARGET_DIR="${TARGET_ROOT}/${REPO_NAME}"

mkdir -p "${TARGET_DIR}"

export CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="${TARGET_DIR}"

exec cargo run -- "$@"
