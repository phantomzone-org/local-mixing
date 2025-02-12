# Local Mixing

## Description

<!-- Add a detailed description of the project here -->

## Commands

### `random-circuit`

Generates a random circuit and saves it to a specified path.

#### Usage

```sh
cargo run --release random-circuit <save_path> <num_wires> <num_gates>
```

#### Arguments

- `<save_path>`: The path where the generated circuit will be saved.
- `<num_wires>`: The number of wires in the circuit.
- `<num_gates>`: The number of gates in the circuit.

### `json`

Loads a circuit from a binary file and optionally saves it as a JSON file.

#### Usage

```sh
cargo run --release json <circuit_path> [json_path]
```

#### Arguments

- `<circuit_path>`: The path to the binary file containing the circuit.
- `[json_path]`: (Optional) The path where the JSON representation of the circuit will be saved. If not provided, the circuit will be printed to the console.

### `local-mixing`

Executes the local mixing job based on a configuration file.

#### Usage

```sh
cargo run --release local-mixing <config_path> [log_path]
```

#### Arguments

- `<config_path>`: The path to the configuration file for the local mixing job.
- `[log_path]`: (Optional) The path where logs will be saved. This argument is required if the `trace` feature is enabled.

### `replace`

Tests the number of samples for a replacement strategy.

#### Usage

```sh
cargo run -- replace <log_path> <strategy> <cf_choice> <n_iter>
```

#### Arguments
- `<log_path>`: The path where logs will be saved.
- `<strategy>`: The replacement strategy to use (as a u8 value). Refer to `cf.rs` for information on how to pick the appropriate u8 value.
- `<cf_choice>`: The control function choice (as a u8 value) Refer to `cf.rs`.
- `<n_iter>`: The number of iterations to run the test.

## Logs

To enable logs, use the following command:

```sh
cargo run --features trace -- <command> [arguments]
```