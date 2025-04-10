#!/bin/bash

cargo run --release build-compression-table bin/table-twobit.db TwoBit 3 9

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Config
WIRES=$(jq -r '.wires' "$SCRIPT_DIR/template-config.json")

# Step 1: Create the "test_outputs" directory if it doesn't exist
mkdir -p .experiments

# Step 2: Generate the current date and time in "YYYY-MM-DD_HH-MM-SS" format
curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

# Step 3: Create the folder with the current date and time within "test_outputs"
BASE_DIR=".experiments/$curr_date_time"
mkdir -p "$BASE_DIR"

mkdir -p $BASE_DIR/inf1
cp $SCRIPT_DIR/inf-config.json $BASE_DIR/inf1/config.json
cargo run --release --features="trace" local-mixing $BASE_DIR/inf1

mkdir -p $BASE_DIR/knd1
cp $SCRIPT_DIR/knd-config.json $BASE_DIR/knd1/config.json
cp $BASE_DIR/inf1/target.json $BASE_DIR/knd1/input.json
cargo run --release --features="trace" local-mixing $BASE_DIR/knd1

mkdir -p $BASE_DIR/inf2
cp $SCRIPT_DIR/inf-config.json $BASE_DIR/inf2/config.json
cp $BASE_DIR/inf1/target.json $BASE_DIR/inf2/input.json
cargo run --release --features="trace" local-mixing $BASE_DIR/inf2

mkdir -p $BASE_DIR/knd2
cp $SCRIPT_DIR/knd-config.json $BASE_DIR/knd2/config.json
cp $BASE_DIR/inf2/target.json $BASE_DIR/knd2/input.json
cargo run --release --features="trace" local-mixing $BASE_DIR/knd2

mkdir -p $BASE_DIR/inf3
cp $SCRIPT_DIR/inf-config.json $BASE_DIR/inf3/config.json
cp $BASE_DIR/inf2/target.json $BASE_DIR/inf3/input.json
cargo run --release --features="trace" local-mixing $BASE_DIR/inf3

mkdir -p $BASE_DIR/knd3
cp $SCRIPT_DIR/knd-config.json $BASE_DIR/knd3/config.json
cp $BASE_DIR/inf3/target.json $BASE_DIR/knd3/input.json
cargo run --release --features="trace" local-mixing $BASE_DIR/knd3
