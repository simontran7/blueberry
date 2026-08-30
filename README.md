<div align="center">
  <img width="170px" src="docs/blueberry-logo.svg">
  <h1>blueberry</h1>
  <p>simple and expressive programming language</p>
</div>

> [!CAUTION]
> The compiler can't compile blueberry programs yet.

## Installation

### Dependencies

- Rust Compiler
- LLVM

### Steps

1. Install Rust
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install LLVM 22
  - On Debian/Ubuntu:
```sh
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 22
sudo apt install libpolly-22-dev
```
  - On macOS:
```sh
brew install llvm@22
echo 'export LLVM_SYS_221_PREFIX="$(brew --prefix llvm@22)"' >> ~/.zshrc
```

3. Git clone the repository

```sh
git clone https://github.com/simontran7/blueberry.git
```

4. `cd` into the `blueberry/` directory, then build the project with GNU Make

```sh
cd blueberry/
cargo build --release
```

5. Move the `target/release/blueberry` binary to a desired location (e.g. in `/Users/<username>`), then add it to your `PATH` by adding the following line to your `.bashrc` file

```sh
# in your .bashrc
export PATH=$PATH:<path to the compiler executable>
```

## Usage

Run `blueberry --help` to see available commands and options.

## Architecture

```mermaid
flowchart TD
  A[Source] -->|Lexical Analysis I| B[List of tokens]
  B -->|Syntactic Analysis| C[Concrete Syntax Tree]
  C -->|Semantic Analysis| D[High-level Intermediate Representation]

  subgraph loop [" "]
    direction TD
    F[Mid-level Intermediate Representation Function] -->|Mid-level Intermediate Representation Transformation Passes| G[Transformed Mid-level Intermediate Representation Function]
    G -->|LLVM IR generation| H[LLVM IR Function]
  end

  D -->|Mid-level Intermediate Representation Construction| loop
  H --> M[(LLVM Module)]
  H -.->|next function| F
```

## Language Reference

See [A Tour of Blueberry](docs/tour-of-blueberry.md)
