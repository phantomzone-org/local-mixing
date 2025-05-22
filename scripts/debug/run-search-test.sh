#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Step 1: Create the "test_outputs" directory if it doesn't exist
mkdir -p .experiments

# Step 2: Generate the current date and time in "YYYY-MM-DD_HH-MM-SS" format
curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

# Step 3: Create the folder with the current date and time within "test_outputs"
BASE_DIR=".experiments/$curr_date_time"
mkdir -p "$BASE_DIR"

cp $SCRIPT_DIR/test-search-config.json "$BASE_DIR/config.json"
cargo run --release search-test $BASE_DIR

