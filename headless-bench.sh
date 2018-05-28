#!/bin/bash

# Wait for other processes to exit before starting the benchmarking
flock -F /tmp/headless-bench.lock -c "sleep 10"

# Example usage:
#     ./headless-bench.sh a7sac pr-4/2018-06-12_14:44:03-a7sac
commit=$1
out_path=$2

# Check out the commit and build a release version
dir_name="alacritty-$commit-$(date '+%N')"
git clone -q https://github.com/chrisduerr/alacritty "$dir_name"
cd "$dir_name"
git checkout "$commit"
cargo build --release
cd ..

xvfb="xvfb-run -a -s '-screen 0 1920x1080x24'"

# List with benchmarks that should be run
# Format:
#     "'bench --mark' 'num bytes' 'out-file-name'"
benchmarks=(\
    "'scrolling' '3000000' 'scrolling'" \
    "'alt-screen-random-write' '150000000' 'alt-screen-random-write'" \
    "'scrolling-in-region --lines-from-bottom 1' '3000000' 'scrolling-in-region-1'" \
    "'scrolling-in-region --lines-from-bottom 50' '3000000' 'scrolling-in-region-50'" \
    "'unicode-random-write' '10000000' 'unicode-random-write'")

# Run all benchmarks with docker
for i in ${!benchmarks[@]}
do
    bench="${benchmarks[$i]}"
    echo "Running benchmark $bench"
    docker_id=$(docker run -d -v "$(pwd):/source" undeadleech/vtebench \
        "cd /source && $xvfb ./$dir_name/target/release/alacritty -e bash ./bench.sh $bench $out_path")
    echo "Exit Code: $(docker wait $docker_id)"
done

# Remove build directory
rm -rf "$dir_name"

