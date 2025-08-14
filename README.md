# cargo-lpatch

A cargo plugin to locally patch dependencies by cloning them and setting up local patches in `.cargo/config.toml`.

## Installation

```bash
cargo install --path .
```

Or install from crates.io (once published):

```bash
cargo install cargo-lpatch
```

## Usage

### Basic Usage

Patch a crate from crates.io:

```bash
cargo lpatch --name serde
```

This will:

1. Query crates.io for the `serde` crate's repository URL
2. Clone the repository to `crates/serde/`
3. Add a local patch configuration to `.cargo/config.toml`

### Custom Clone Directory

Specify a custom directory for cloning:

```bash
cargo lpatch --name serde --dir my-dependencies
```

### Direct Git URL

You can also provide a direct git URL instead of a crate name:

```bash
cargo lpatch --name https://github.com/serde-rs/serde.git
```

or

```bash
cargo lpatch --name git@github.com:serde-rs/serde.git
```

## How It Works

1. **Crate Resolution**: If you provide a crate name, the tool queries crates.io API to get the repository URL. If you provide a git URL, it uses that directly.

2. **Repository Cloning**: The tool clones the repository to the specified directory (default: `crates/`).

3. **Configuration Update**: The tool creates or updates `.cargo/config.toml` with a local patch configuration pointing to the cloned repository.

## Generated Configuration

After running the tool, your `.cargo/config.toml` will contain something like:

```toml
[patch.crates-io]
serde = { path = "crates/serde" }
```

This tells Cargo to use the local version of the crate instead of downloading it from crates.io.

## Examples

### Patch multiple crates

```bash
cargo lpatch --name serde --dir deps
cargo lpatch --name tokio --dir deps
cargo lpatch --name clap --dir deps
```

### Working with private repositories

```bash
cargo lpatch --name git@github.com:myorg/private-crate.git --dir vendor
```

## Features

- ✅ Query crates.io for repository URLs
- ✅ Support for git URLs (https, ssh, git://)
- ✅ Automatic `.cargo/config.toml` management
- ✅ Custom clone directories
- ✅ Progress indication for clone operations
- ✅ Update existing clones with `git pull`
- ✅ Smart URL cleaning and validation

## Requirements

- Rust 1.70+
- Git installed and available in PATH
- Network access for crates.io queries and git operations

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
