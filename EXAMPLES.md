# Test examples

## Example 1: Testing with a popular crate

```bash
# This will clone serde from its repository and set up local patch
cargo lpatch --name serde

# The result should be:
# 1. Directory `crates/serde/` created with the cloned repository
# 2. `.cargo/config.toml` updated with the local patch configuration
```

## Example 2: Custom directory

```bash
# Clone to a custom directory
cargo lpatch --name tokio --dir my-deps
```

## Example 3: Direct git URL

```bash
# Use a direct git URL
cargo lpatch --name https://github.com/serde-rs/serde.git
```

## Expected `.cargo/config.toml` output

After running the commands above, your `.cargo/config.toml` should look like:

```toml
[patch.crates-io]
serde = { path = "crates/serde" }
tokio = { path = "my-deps/tokio" }
```

## Testing the patch

After setting up the local patch, you can:

1. Make changes to the code in the cloned repository
2. Your project will automatically use the local version instead of the crates.io version
3. Test your changes without publishing

## Reverting changes

To remove a local patch:

1. Remove the entry from `.cargo/config.toml`
2. Optionally delete the cloned directory
