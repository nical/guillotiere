#!/bin/sh

mkdir -p ./tmp
cargo build
cargo run -q -- init 1024 1024 --atlas tmp/worst-case.ron
cargo run -q -- allocate 10 10 --atlas tmp/worst-case.ron
cargo run -q -- allocate 11 11 --atlas tmp/worst-case.ron
cargo run -q -- allocate 12 12 --atlas tmp/worst-case.ron
cargo run -q -- allocate 13 13 --atlas tmp/worst-case.ron
cargo run -q -- allocate 14 14 --atlas tmp/worst-case.ron
cargo run -q -- allocate 15 15 --atlas tmp/worst-case.ron
cargo run -q -- allocate 16 16 --atlas tmp/worst-case.ron
cargo run -q -- allocate 17 17 --atlas tmp/worst-case.ron
cargo run -q -- allocate 18 18 --atlas tmp/worst-case.ron
cargo run -q -- allocate 19 19 --atlas tmp/worst-case.ron
cargo run -q -- allocate 20 20 --atlas tmp/worst-case.ron
cargo run -q -- allocate 21 21 --atlas tmp/worst-case.ron
cargo run -q -- allocate 22 22 --atlas tmp/worst-case.ron
cargo run -q -- allocate 23 23 --atlas tmp/worst-case.ron
cargo run -q -- allocate 24 24 --atlas tmp/worst-case.ron
cargo run -q -- svg tmp/worst-case.svg --atlas tmp/worst-case.ron