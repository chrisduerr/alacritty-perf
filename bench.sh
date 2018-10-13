#!/bin/bash

# Wait for other processes to exit before starting the benchmarking
lock="/tmp/alacritty-bench.pid"
exec 200>$lock
flock 200

# Example usage:
#     ./headless-bench.sh a7sac pr-4/2018-06-12_14:44:03-a7sac
commit=$1
out_path=$2
regex='s/^test \([^ ]*\).*bench: *\([0-9,]*\).* \([0-9,]*\).$/\1;\2;\3/'

# Check out the commit
dir_name="alacritty-$commit-$(date '+%N')"
git clone --quiet https://github.com/chrisduerr/alacritty "$dir_name"
cd "$dir_name"
git reset --hard --quiet "$commit"

# Benchmark this commit
cargo bench --features bench 2> /dev/null
for bench in $(ls "./target/criterion"); do
    mkdir -p "$out_path"
    cp "./target/criterion/$bench/new/estimates.json" "$out_path/$bench" || true
done

# Remove build directory
cd ..
rm -rf "$dir_name"
