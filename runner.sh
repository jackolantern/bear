#!/usr/bin/bash

# export RUSTFLAGS="--cfg debug"
export RUST_BACKTRACE=full

cargo run --bin bear-ass -- $1.bear $1.bin -d && cargo run --bin bear-app -- $1 ${@:2}

