#!/bin/bash

# Wait for other processes to exit before starting the benchmarking
lock="/tmp/headless-bench.pid"
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
result=$(cargo bench --features bench 2> /dev/null | grep "... bench:" | sed "$regex")
cd ..
for bench in $(echo -e "$result"); do
    name=$(echo "$bench" | sed 's/\([^;]*\);.*/\1/')
    avg=$(echo "$bench" | sed 's/.*;\(.*\);.*/\1/' | sed 's/,//')
    # # dev=$(echo "$bench" | sed 's/.*;\(.*\)/\1/' | sed 's/,//')
    mkdir -p "$out_path"
    echo "$avg" > "$out_path/$name"
done

# Remove build directory
rm -rf "$dir_name"
