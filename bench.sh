#!/bin/bash

# Example usage:
#     ./bench.sh scrolling 5000000 scrolling a7sac3cashas39sac3810
bench=$1
bytes=$2
name=$3
commit=$4

# Generate requested benchmark
vtebench -w $(tput cols) -h $(tput lines) -sb $bytes $bench > "/$name.vte"

# Create required directories
mkdir -p "/source/results/$commit"

# Run the benchmark and write output to `$BENCH.md`
hyperfine --print-stdout --export-markdown "/source/$name.md" "cat /$name.vte"

