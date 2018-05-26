#!/bin/bash

# Simple helper for starting the actix server
cargo build --release &>> /tmp/alacritty-perf.log
nohup cargo run --release &>> /tmp/alacritty-perf.log &
