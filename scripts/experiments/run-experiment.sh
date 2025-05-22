#!/bin/bash

# Uncomment to generate compression table
# cargo run --release build-compression-table bin/table-twobit.db TwoBit 3 9

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p .experiments

curr_date_time=$(date +"%Y-%m-%d_%H-%M-%S")

BASE_DIR=".experiments/$curr_date_time"
mkdir -p $BASE_DIR/inflationary
cp $SCRIPT_DIR/inf-config.json $BASE_DIR/inflationary/config.json

### Generate input.json

# Uncomment for identity-inflation
cargo run --release --features="trace" random-inflated
mv original.json $BASE_DIR/inflationary
mv input.json $BASE_DIR/inflationary

# Uncomment for just inflationary stage
cargo run --release --features="trace" random-circuit input.json 16 256 TwoBit
mv input.json $BASE_DIR/inflationary

# echo "Inflationary stage"
# cargo run --release --features="trace" local-mixing $BASE_DIR/inflationary
# cargo run --release --features="trace" equiv $BASE_DIR/inflationary/input.json $BASE_DIR/inflationary/target.json 10000

# echo "Kneading stages"
mkdir -p $BASE_DIR/kneading
cargo run --release --features="trace" experiment $BASE_DIR


# cp $SCRIPT_DIR/configs/inf-config.json $BASE_DIR/inf1/config.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/inf1

# mkdir -p $BASE_DIR/knd1
# cp $SCRIPT_DIR/configs/knd-config.json $BASE_DIR/knd1/config.json
# cp $BASE_DIR/inf1/target.json $BASE_DIR/knd1/input.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/knd1

# mkdir -p $BASE_DIR/inf2
# cp $SCRIPT_DIR/configs/inf-config.json $BASE_DIR/inf2/config.json
# cp $BASE_DIR/inf1/target.json $BASE_DIR/inf2/input.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/inf2

# mkdir -p $BASE_DIR/knd2
# cp $SCRIPT_DIR/configs/knd-config.json $BASE_DIR/knd2/config.json
# cp $BASE_DIR/inf2/target.json $BASE_DIR/knd2/input.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/knd2

# mkdir -p $BASE_DIR/inf3
# cp $SCRIPT_DIR/configs/inf-config.json $BASE_DIR/inf3/config.json
# cp $BASE_DIR/inf2/target.json $BASE_DIR/inf3/input.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/inf3

# mkdir -p $BASE_DIR/knd3
# cp $SCRIPT_DIR/configs/knd-config.json $BASE_DIR/knd3/config.json
# cp $BASE_DIR/inf3/target.json $BASE_DIR/knd3/input.json
# cargo run --release --features="trace" local-mixing $BASE_DIR/knd3
