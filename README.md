<div align="center">
  <img width="170px" src="website/blueberry-logo.svg">
  <h1>blueberry</h1>
  <p>simple and expressive programming language</p>
</div>

## Building

### Requirements

- Rust
- LLVM 22

If `llvm-config` (from LLVM 22) isn't on your `PATH`, which is the case by default with Homebrew and `apt`, point the build at LLVM's install directory with `LLVM_SYS_221_PREFIX`:

```bash
# macOS (brew install llvm@22)
export LLVM_SYS_221_PREFIX="$(brew --prefix llvm@22)"

# Ubuntu/Debian (apt install llvm-22-dev libpolly-22-dev)
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22
```

To set it once for every `cargo` command, add it to `~/.cargo/config.toml` instead:

```toml
[env]
LLVM_SYS_221_PREFIX = "/usr/lib/llvm-22"  # or the output of `brew --prefix llvm@22`
```

### Steps

1. Git clone the repository

```bash
git clone https://github.com/simontran7/blueberry.git
```

2. Change directory 

```bash
cd blueberry/
```

3. Build

```bash
cargo build
```

## Website

The website is built with `mdbook`. 

```bash
mdbook serve website --open
```

## Docs

See [docs/](docs/README.md) for the compiler's architecture, how to test it, and its style guide.
