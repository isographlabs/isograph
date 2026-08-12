# Config panic message and u64 newtype Display

Two independent one-function fixes, each its own commit.

## Change 1: `create_config` panics through `display()`

`create_config` branches on `config_location.to_str()` only to choose between two panic messages, dropping the path from the message when it is not UTF-8. `Path::display` prints any path, so one message covers both cases and the branch disappears.

```rust
// from crates/isograph_config/src/compilation_options.rs (before)
    let config_contents = match std::fs::read_to_string(config_location) {
        Ok(contents) => contents,
        Err(_) => match config_location.to_str() {
            Some(loc) => {
                panic!("Expected config to be found at {loc}")
            }
            None => {
                panic!("Expected config to be found.")
            }
        },
    };
```

```rust
// from crates/isograph_config/src/compilation_options.rs (after)
    let config_contents = std::fs::read_to_string(config_location).unwrap_or_else(|_| {
        panic!(
            "Expected config to be found at {}",
            config_location.display()
        )
    });
```

The panic itself is pre-existing behavior at the CLI edge and stays; only the message construction changes. No test accompanies this change: the function reads the real filesystem, and the change is message formatting on a panic path.

## Change 2: `u64_newtype!` Display prints the type name

The generated `Display` impl interpolates `$named` inside a string literal, where macro variables do not substitute, so every generated type prints as the literal text `$named(42)`. `stringify!` produces the type name.

```rust
// from crates/u64_newtypes/src/lib.rs (before)
        impl std::fmt::Display for $named {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("$named({})", self.0))
            }
        }
```

```rust
// from crates/u64_newtypes/src/lib.rs (after)
        impl std::fmt::Display for $named {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("{}({})", stringify!($named), self.0))
            }
        }
```

The one invocation in the workspace is `u64_newtype!(HashKey)` in `crates/pico/src/intern.rs`; `HashKey` gains the intended `HashKey(42)` form. Nothing formats a `HashKey` with `Display` today, so no output changes anywhere.

The crate has no tests; this adds the first one. The macro expansion references `serde::Serialize` and `serde::Deserialize`, so invoking the macro inside the crate requires serde as a dev-dependency:

```toml
# from crates/u64_newtypes/Cargo.toml (after)
[dev-dependencies]
serde = { workspace = true }
```

```rust
// from crates/u64_newtypes/src/lib.rs
#[cfg(test)]
mod test {
    crate::u64_newtype!(ExampleId);

    #[test]
    fn display_prints_the_type_name_and_value() {
        assert_eq!(ExampleId(42).to_string(), "ExampleId(42)");
    }
}
```

## Landing checklist

- `cargo test -p u64_newtypes` passes.
- `cargo build -p isograph_config` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
