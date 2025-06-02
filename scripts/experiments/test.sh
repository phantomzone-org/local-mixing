#!/bin/bash

CMD_ARG=$1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p ".experiments/test-$CMD_ARG"

curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

BASE_DIR=".experiments/test-$CMD_ARG/$curr_date_time"
mkdir -p $BASE_DIR
cp $SCRIPT_DIR/test-$CMD_ARG-config.json $BASE_DIR/config.json

FEATURES="trace,search-3"

cargo run --release --features="$FEATURES" local-mixing $BASE_DIR
cargo run --release --features="$FEATURES" distinguisher $BASE_DIR/input.json $BASE_DIR/inflationary.json 100 $BASE_DIR/data-inf.json
cargo run --release --features="$FEATURES" distinguisher $BASE_DIR/input.json $BASE_DIR/target.json 100 $BASE_DIR/data-knd.json

cd plot
source venv/bin/Activate
python scripts/heatmap.py ../$BASE_DIR/data-inf.json ../$BASE_DIR/heatmap-inf.png
python scripts/heatmap.py ../$BASE_DIR/data-knd.json ../$BASE_DIR/heatmap-knd.png
