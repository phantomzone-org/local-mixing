#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Config
WIRES=$(jq -r '.wires' "$SCRIPT_DIR/template-config.json")

# Step 1: Create the "test_outputs" directory if it doesn't exist
mkdir -p .test_outputs

# Step 2: Generate the current date and time in "YYYY-MM-DD_HH-MM-SS" format
curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

# Step 3: Create the folder with the current date and time within "test_outputs"
BASE_DIR=".test_outputs/$curr_date_time"
mkdir -p "$BASE_DIR"

for i in {1..5}; do
    echo "Iteration $i:"
    CURR_DIR="$BASE_DIR/$i"
    mkdir -p $CURR_DIR
    cp $SCRIPT_DIR/template-config.json "$CURR_DIR/config.json"
    cargo run --release random-circuit $CURR_DIR/input.bin $WIRES 1000
    cargo run --release --features="correctness,trace" local-mixing $CURR_DIR
    echo "Completed iteration $i."
done
