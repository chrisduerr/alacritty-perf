#!/bin/bash

# Example usage:
#     ./bench.sh scrolling 5000000 scrolling pr-4/a7sac3cashas39sac3810
bench=$1
bytes=$2
name=$3
out_path=$4

# Generate requested benchmark
vtebench -w $(tput cols) -h $(tput lines) -sb $bytes $bench > "/$name.vte"

# Create required directories
mkdir -p "/source/results/$out_path"

# Run the benchmark and write output to `$BENCH.md`
hyperfine --print-stdout --export-json "/source/results/$out_path/$name.json" --export-markdown "/source/results/$out_path/$name.md" "cat /$name.vte"

# Convert markdown to html
markdown -o "/source/results/$out_path/$name.html" "/source/results/$out_path/$name.md"

