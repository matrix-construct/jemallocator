#!/usr/bin/env sh

set -ex

export RUSTDOCFLAGS="--cfg jemallocator_docs"
cargo doc --all-features
cargo doc -p tikv-jemalloc-sys
cargo doc -p tikv-jemalloc-ctl
