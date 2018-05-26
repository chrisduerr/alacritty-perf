#!/bin/bash

# Simple helper for starting the actix server
cargo build --release 2&>> /tmp/alacritty-perf.log
RUST_LOG=info nohup cargo run --release 2&>> /tmp/alacritty-perf.log &
