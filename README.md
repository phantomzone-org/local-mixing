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

- `<save_path>`: The path where the generated circuit will be saved.
- `<num_wires>`: The number of wires in the circuit.
- `<num_gates>`: The number of gates in the circuit.

#### `local-mixing`

Executes the local mixing job based on a configuration file.

#### Usage

```sh
cargo run --release local-mixing <config-dir>
```

- `<config_dir>`: Path to the job directory. Should already include `config.json` and `input.bin`.

An example config:
```json
{
  "wires": 64,
  "inflationary_stage_steps": 1,
  "kneading_stage_steps": 1,
  "max_replacement_samples": 10000000,
  "max_attempts_without_success": 100,
  "save": true,
  "epoch_size": 1000
}
```
Setting `save` to true requires that `epoch_size` is also specified. If set, every `epoch_size` steps the current circuit and no. steps will be saved.

#### `json`

Loads a circuit from a binary file and optionally saves it as a JSON file.

#### Usage

```sh
cargo run --release json <circuit_path> [json_path]
```

- `<circuit_path>`: The path to the binary file containing the circuit.
- `[json_path]`: (Optional) The path where the JSON representation of the circuit will be saved. If not provided, the circuit will be printed to the console.

#### `equiv`

Tests that two circuits are functionally equivalent (probabilistic test).

#### Usage
```sh
cargo run equiv <circuit_one_path> <circuit_two_path> <iter>
```
- `<circuit_one_path>` and `<circuit_two_path>`: paths to the two circuits.
- `<iter>`: number of random bitstrings circuit one and two are compared against.

#### `replace`

Tests the number of samples for a replacement strategy.

#### Usage

```sh
cargo run -- replace <log_path> <strategy> <cf_choice> <n_iter>
```

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