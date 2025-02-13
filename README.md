# Local Mixing

Test implementation of the local mixing procedure. Critical components of this repo are:

- `src/local-mixing/`: Running and debugging local mixing, including searching for candidate convex-connected subsets, permuting and updating the circuit. Runs inflationary and kneading stages.
- `src/replacement/`: Computes sampling small random circuits and computes replacements.
- `benches/`: Benchmarks for search and replacement.

## Commands

#### `random-circuit`

Generates a random circuit and saves it to a specified path.

#### Usage

```sh
cargo run random-circuit <save_path> <num_wires> <num_gates>
```

#### Arguments

- `<save_path>`: The path where the generated circuit will be saved.
- `<num_wires>`: The number of wires in the circuit.
- `<num_gates>`: The number of gates in the circuit.

#### `local-mixing`

Executes the local mixing job based on a configuration file.

#### Usage

```sh
cargo run --release local-mixing <config_path> [log_path]
```

#### Arguments

- `<config_path>`: The path to the configuration file for the local mixing job.
- `[log_path]`: (Optional) The path where logs will be saved. Ignored unless the "trace" or "time" features are enabled.

#### `json`

Loads a circuit from a binary file and optionally saves it as a JSON file.

#### Usage

```sh
cargo run --release json <circuit_path> [json_path]
```

#### Arguments

- `<circuit_path>`: The path to the binary file containing the circuit.
- `[json_path]`: (Optional) The path where the JSON representation of the circuit will be saved. If not provided, the circuit will be printed to the console.

#### `replace`

Tests the number of samples for a replacement strategy.

#### Usage

```sh
cargo run -- replace <log_path> <strategy> <cf_choice> <n_iter>
```

#### Arguments

- `<log_path>`: The path where logs will be saved.
- `<strategy>`: The replacement strategy to use (as a u8 value). Refer to `cf.rs` for information on how to pick the appropriate u8 value.
- `<cf_choice>`: The control function choice (as a u8 value). Refer to `cf.rs`.
- `<n_iter>`: The number of iterations to run the test.

## Features

To enable features, e.g.:

```sh
cargo run --release --features "trace correctness" local-mixing ...
```
- `trace` enables logs of each step of local-mixing. Logs will be saved to the log path.
- `time` times the finding-replacement step, and logs it every 1000 steps.
- `correctness` asserts that after each step, the current job circuit is functionally equivalent to the input circuit (not save). This runs a probabilistic test. Warning: doing this slows the execution down significantly.