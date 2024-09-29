A Rust procedural macro for automatically generating structs from configuration files (JSON, YAML, TOML).

## Example Usage

1. Create a configuration file (e.g., `config.yaml`).
2. Use the `generate` attribute macro:

```yaml
server:
  port: 8080
features:
  - logging
  - authentication
```

Generate and use the struct:

```rust
#[generate("config.yaml")]
struct AppConfig;

fn main() {
    let config = AppConfig::new();
    println!("Server port: {}", config.server.port);
    println!("Features: {:?}", config.features);
}
```

## API

- `new()`: Creates a new instance with values from the file.
- `default()`: Creates a new instance no values from the file.
- `is_empty()`: Returns `true` if all fields are default values.
