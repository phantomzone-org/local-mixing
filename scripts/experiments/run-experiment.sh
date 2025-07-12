#!/bin/bash

# Uncomment to generate compression table
# cargo run --release build-compression-table bin/table-twobit.db TwoBit 3 9

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p .experiments

curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

BASE_DIR=".experiments/$curr_date_time"
mkdir -p $BASE_DIR
cp $SCRIPT_DIR/run-experiment.sh $BASE_DIR/script.sh

mkdir -p $BASE_DIR/inflationary

### Generate input.json

# Uncomment for identity-inflation
# cargo run --release --features="trace" random-inflated
# mv original.json $BASE_DIR/inflationary
# mv input.json $BASE_DIR/inflationary
# mkdir -p $BASE_DIR/0
# cp $SCRIPT_DIR/knd-config.json $BASE_DIR/0/config.json
# cp $BASE_DIR/inflationary/input.json $BASE_DIR/0/input.json

# Uncomment for just inflationary stage
# cargo run --release --features="trace" random-circuit input.json 16 256 TwoBit
# mv input.json $BASE_DIR/inflationary
# cp $SCRIPT_DIR/inf-config.json $BASE_DIR/inflationary/config.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/inflationary
# mkdir -p $BASE_DIR/0
# cp $SCRIPT_DIR/knd-config.json $BASE_DIR/0/config.json
# cp $BASE_DIR/inflationary/target.json $BASE_DIR/0/input.json

# Uncomment for block identity inflation
cargo run --release --features="trace" random-inflated-block
mv original.json $BASE_DIR/inflationary
mv input.json $BASE_DIR/inflationary
mkdir -p $BASE_DIR/0
cp $SCRIPT_DIR/knd-config.json $BASE_DIR/0/config.json
cp $BASE_DIR/inflationary/input.json $BASE_DIR/0/input.json

for i in {0..20}; do
    cargo run --release --features="trace" local-mixing $BASE_DIR/$i
    mkdir -p $BASE_DIR/$((i + 1))
    cp $SCRIPT_DIR/knd-config.json $BASE_DIR/$((i + 1))/config.json
    cp $BASE_DIR/$i/target.json $BASE_DIR/$((i + 1))/input.json
done
