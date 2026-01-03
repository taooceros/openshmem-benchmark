# OpenSHMEM Benchmark Environment Setup

This project uses [devenv](https://devenv.sh) to manage its development environment, ensuring a reproducible setup with all necessary dependencies like Rust (nightly), GCC 15, and other tools.

## Prerequisites

### 1. Install Nix

You need to have Nix installed. If you haven't installed it yet, run:

```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
```

### 2. Install Devenv

Install `devenv` using Nix:

```bash
nix profile install --accept-flake-config 'github:cachix/devenv#devenv'
```

### 3. Install Direnv (Optional but Recommended)

`direnv` automatically activates the environment when you enter the directory.

```bash
nix profile install nixpkgs#direnv
```

Hook it into your shell (add to your `~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`). See [direnv documentation](https://direnv.net/docs/hook.html) for details.

## Usage

### Entering the Environment

If you have `direnv` installed and configured:

```bash
direnv allow
```

Otherwise, you can manually enter the shell:

```bash
devenv shell
```

### Running the Benchmark

The `devenv.nix` file defines a `run` script. You can execute the benchmark runner (NuShell script) directly:

```bash
devenv run
```

Or if you are inside the shell:

```bash
nu run.nu
```

## Running Benchmarks

The `run.nu` script provides several commands to run different types of benchmarks.

### Environment Variables

Before running, you may want to configure the following environment variables:

- `PEER`: The hostname of the second peer for distributed benchmarks (default: `venus`).
- `PEER_CWD`: The working directory on the peer machine (default: `openshmem-benchmark`).
- `DEVICE`: The InfiniBand/network device to use (default: `mlx5_1:1`).

### Available Commands

#### Run All Benchmarks

To run the full suite of benchmarks (RMA, Atomic, Latency, Broadcast, All-to-all, YCSB, Trace):

```bash
nu run.nu bench
```

#### Run Specific Benchmarks

You can run specific categories of benchmarks:

- **RMA (Remote Memory Access):**
  ```bash
  nu run.nu bench rma
  ```
- **Atomic Operations:**
  ```bash
  nu run.nu bench atomic
  ```
- **Latency:**
  ```bash
  nu run.nu bench latency
  ```
- **Broadcast:**
  ```bash
  nu run.nu bench broadcast
  ```
- **All-to-All:**
  ```bash
  nu run.nu bench alltoall
  ```
- **YCSB Workloads:**
  ```bash
  nu run.nu bench ycsb
  ```
- **Trace Replay:**
  ```bash
  nu run.nu bench trace
  ```

#### Manual / Single Test Run

For debugging or running a specific configuration, use the `test` command:

```bash
nu run.nu test [flags]
```

**Options:**

- `--epoch_size` / `-e`: Size of the epoch (default: 16).
- `--data_size` / `-s`: Size of data payload in bytes (default: 8).
- `--iterations` / `-i`: Number of iterations (default: 10000).
- `--duration`: Duration of the test in seconds (default: 10).
- `--num_pe`: Number of Processing Elements (PEs) (default: 1).
- `--num_working_set`: Working set size (default: 1).
- `--latency`: Enable latency measurement.
- `--operation` / `-o`: Specify the operation as a record (default: `{ "group": "range", "operation": "put" }`).

**Example:**

```bash
nu run.nu test --data_size 1024 --num_pe 2
```

#### Profiling

To run a profiling session with flamegraph support:

```bash
nu run.nu profile
```

## Environment Details

The environment provides:
- **Rust Toolchain**: Nightly (defined in `rust-toolchain.toml`).
- **Compilers**: GCC 15, Libclang.
- **Shell**: NuShell.
- **Environment Variables**:
    - `PEER`: Defaults to "venus".
    - `PEER_CWD`: Defaults to "openshmem-benchmark".
    - `LIBCLANG_PATH`: Set automatically for bindgen/clang usage.
