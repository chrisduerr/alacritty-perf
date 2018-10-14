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
echo "Cloning '$commit'"
dir_name="alacritty-$commit-$(date '+%N')"
git clone --quiet https://github.com/jwilm/alacritty "$dir_name"
cd "$dir_name"
git reset --hard --quiet "$commit"

# Benchmark this commit
echo "Running benchmarks"
cargo bench --features bench &> /dev/null
echo "Copying benchmarks"
cd ..
mkdir -p "$out_path"
for bench in $(ls "$dir_name/target/criterion"); do
    cp "$dir_name/target/criterion/$bench/new/estimates.json" "$out_path/$bench" && \
        echo "    Copied '$bench'" || echo "    Unable to copy '$bench'"
done

# Remove build directory
echo "Removing build artifacts"
rm -rf "$dir_name"
