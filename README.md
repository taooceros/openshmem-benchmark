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

## Environment Details

The environment provides:
- **Rust Toolchain**: Nightly (defined in `rust-toolchain.toml`).
- **Compilers**: GCC 15, Libclang.
- **Shell**: NuShell.
- **Environment Variables**:
    - `PEER`: Defaults to "venus".
    - `PEER_CWD`: Defaults to "openshmem-benchmark".
    - `LIBCLANG_PATH`: Set automatically for bindgen/clang usage.
