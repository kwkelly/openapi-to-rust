# openapi-to-rust

[![CI](https://github.com/gpu-cli/openapi-to-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/gpu-cli/openapi-to-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/openapi-to-rust.svg)](https://crates.io/crates/openapi-to-rust)
[![docs.rs](https://docs.rs/openapi-to-rust/badge.svg)](https://docs.rs/openapi-to-rust)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A Rust code generator that creates strongly-typed structs, HTTP clients, SSE streaming clients, and axum server handlers from OpenAPI 3.1 specifications.

We originally built this internally at [GPU CLI](https://gpu-cli.sh) to generate typed Rust clients for OpenAI, Anthropic, and other APIs. After battle-testing it against real-world specs with complex union types, discriminated enums, and streaming endpoints, we decided to open source it.

## Features

- **OpenAPI 3.1 support** — objects, arrays, enums, `oneOf`, `anyOf`, `allOf`, discriminated unions
- **HTTP client generation** — async clients with retry logic, tracing, and auth middleware
- **SSE streaming clients** — first-class Server-Sent Events support with reconnection
- **Axum handler generation** — server-side handler stubs and traits for building APIs
- **Smart `$ref` resolution** — handles circular references and deep nesting
- **Discriminator detection** — auto-detects tagged unions from `oneOf`/`anyOf` with const properties
- **TOML configuration** — declarative config as an alternative to the Rust API
- **Snapshot testing** — built-in `insta` snapshot tests for generated output
- **Optional [specta](https://github.com/oscartbeaumont/specta) support** — add `specta::Type` derives for TypeScript codegen

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
openapi-to-rust = "0.1"

# For axum handler generation, enable the feature:
# openapi-to-rust = { version = "0.1", features = ["axum-handlers"] }
```

Or install the CLI:

```bash
cargo install openapi-to-rust
```

## Quick Start

### CLI (TOML config)

Create `openapi-to-rust.toml`:

```toml
[generator]
spec_path = "openapi.json"
output_dir = "src/generated"
module_name = "api"

[features]
enable_async_client = true
enable_axum_handlers = false  # Set to true to generate server handlers

[http_client]
base_url = "https://api.example.com"
timeout_seconds = 30

[http_client.retry]
max_retries = 3

[http_client.auth]
type = "Bearer"
header_name = "Authorization"
```

Then generate:

```bash
openapi-to-rust generate --config openapi-to-rust.toml
```

### Library API

```rust
use openapi_to_rust::{SchemaAnalyzer, CodeGenerator, GeneratorConfig};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec_content = std::fs::read_to_string("openapi.json")?;
    let spec_value: serde_json::Value = serde_json::from_str(&spec_content)?;

    let mut analyzer = SchemaAnalyzer::new(spec_value)?;
    let mut analysis = analyzer.analyze()?;

    let config = GeneratorConfig {
        spec_path: PathBuf::from("openapi.json"),
        output_dir: PathBuf::from("src/generated"),
        module_name: "api".to_string(),
        enable_sse_client: true,
        enable_async_client: true,
        enable_axum_handlers: true,  // Generate server handlers
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);
    let result = generator.generate_all(&mut analysis)?;
    generator.write_files(&result)?;

    println!("Generated {} files", result.files.len());
    Ok(())
}
```

## Generated Output

The generator produces up to five files:

| File | Description |
|------|-------------|
| `types.rs` | All struct/enum definitions from OpenAPI schemas |
| `client.rs` | Async HTTP client with typed methods per operation |
| `streaming.rs` | SSE streaming client with event parsing |
| `handlers.rs` | Axum handler trait and router builder (optional) |
| `mod.rs` | Module declarations |

### Generated Client Usage

```rust
use crate::generated::client::HttpClient;
use crate::generated::types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new()
        .with_base_url("https://api.example.com")
        .with_api_key("your-api-key");

    let request = CreateResourceRequest { /* ... */ };
    let response = client.create_resource(request).await?;
    Ok(())
}
```

## Axum Handler Generation

Enable the `axum-handlers` feature to generate server-side handler stubs:

```toml
[dependencies]
openapi-to-rust = { version = "0.1", features = ["axum-handlers"] }
```

### Generated Handler Structure

The generator creates:

1. **`ApiHandlers` trait** — Defines all endpoint handlers as async methods
2. **Parameter structs** — `PathParams` and `QueryParams` for each operation
3. **`ApiError` enum** — Error type with `IntoResponse` implementation
4. **`create_router` function** — Builds an axum `Router` from your implementation

### Example Generated Code

For an OpenAPI spec with:

```yaml
paths:
  /users/{id}:
    get:
      operationId: getUser
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
```

The generator produces:

```rust
// Parameter struct
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetUserPathParams {
    pub id: String,
}

// Handler trait
#[async_trait::async_trait]
pub trait ApiHandlers: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn get_user(
        &self,
        Path(params): Path<GetUserPathParams>,
    ) -> Result<Json<User>, Self::Error>;
}

// Router builder
pub fn create_router<H>(handler: H) -> Router<H>
where
    H: ApiHandlers + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/users/{id}", get(handle_get_user))
        .with_state(handler)
}
```

### Implementing the Handlers

```rust
use api::handlers::{ApiHandlers, create_router, ApiError};
use api::types::User;
use axum::{extract::Path, Json};

struct MyHandler {
    db: Database,
}

#[async_trait::async_trait]
impl ApiHandlers for MyHandler {
    type Error = ApiError;

    async fn get_user(
        &self,
        Path(params): Path<GetUserPathParams>,
    ) -> Result<Json<User>, Self::Error> {
        let user = self.db.find_user(&params.id).await
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", params.id)))?;
        Ok(Json(user))
    }
}

#[tokio::main]
async fn main() {
    let handler = MyHandler { db: Database::new() };
    let app = create_router(handler);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Handler Features

- **Automatic parameter extraction** — Path, query, and header parameters mapped to axum extractors
- **Type-safe request bodies** — JSON bodies use generated types from `types.rs`
- **Error handling** — Built-in `ApiError` enum with HTTP status code mapping
- **Router generation** — Automatic route registration with correct HTTP methods

## HTTP Client Features

The generated HTTP client includes:

- **All HTTP methods** — GET, POST, PUT, DELETE, PATCH
- **Retry with backoff** — exponential backoff via `reqwest-retry` (retries 429, 500, 502, 503, 504)
- **Distributed tracing** — automatic request/response spans via `reqwest-tracing`
- **Authentication** — Bearer token, API key, or custom header auth
- **Default headers** — configurable per-client headers
- **Middleware stack** — composable request/response middleware

## SSE Streaming

First-class support for Server-Sent Events endpoints:

```rust
use openapi_to_rust::streaming::*;

let streaming_config = StreamingConfig {
    endpoints: vec![StreamingEndpoint {
        operation_id: "createChatCompletion".to_string(),
        path: "chat/completions".to_string(),
        stream_parameter: "stream".to_string(),
        event_union_type: "ChatCompletionStreamEvent".to_string(),
        event_flow: EventFlow::StartDeltaStop {
            start_events: vec!["response.created".to_string()],
            delta_events: vec!["response.output_text.delta".to_string()],
            stop_events: vec!["response.completed".to_string()],
        },
        ..Default::default()
    }],
    reconnection_config: Some(ReconnectionConfig {
        max_retries: 5,
        initial_delay_ms: 500,
        max_delay_ms: 16000,
        backoff_multiplier: 2.0,
    }),
    ..Default::default()
};
```

## Discriminated Unions

Automatically detects and generates tagged enums from OpenAPI discriminator patterns:

```yaml
# OpenAPI spec
MessageContent:
  oneOf:
    - $ref: '#/components/schemas/TextContent'
    - $ref: '#/components/schemas/ImageContent'
  discriminator:
    propertyName: type
    mapping:
      text: '#/components/schemas/TextContent'
      image: '#/components/schemas/ImageContent'
```

Generates:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
}
```

Also handles untagged unions (`anyOf` without discriminator), `allOf` composition, extensible enums, and inline variant schemas.

## TOML Configuration Reference

```toml
[generator]
spec_path = "openapi.json"              # Path to OpenAPI spec (required)
output_dir = "src/generated"            # Output directory (required)
module_name = "types"                   # Module name (required)

[features]
enable_sse_client = true                # Generate SSE streaming client
enable_async_client = true              # Generate HTTP REST client
enable_specta = false                   # Add specta::Type derives
enable_axum_handlers = false            # Generate axum server handlers

[http_client]
base_url = "https://api.example.com"
timeout_seconds = 30                    # 1-3600

[http_client.retry]
max_retries = 3                         # 0-10
initial_delay_ms = 500                  # 100-10000
max_delay_ms = 16000                    # 1000-300000

[http_client.tracing]
enabled = true

[http_client.auth]
type = "Bearer"                         # Bearer | ApiKey | Custom
header_name = "Authorization"

[[http_client.headers]]
name = "content-type"
value = "application/json"

[nullable_overrides]
"Response.error" = true                 # Override nullable detection

[type_mappings]
"DateTime" = "chrono::DateTime<chrono::Utc>"
```

## Testing

```bash
# Run all tests
cargo test

# Run with axum feature enabled
cargo test --features axum-handlers

# Run snapshot tests
cargo insta test

# Review snapshot changes
cargo insta review
```

## Examples

The `examples/` directory contains working examples for:

- Basic type generation
- HTTP client generation
- Axum handler generation
- Discriminated unions and `anyOf`/`oneOf` patterns
- `allOf` composition
- Inline objects and enums
- Nullable field handling
- Recursive schemas
- TOML configuration

```bash
cargo run --example basic_generation
cargo run --example client_generation_example
cargo run --example axum_handler_generation --features axum-handlers
cargo run --example discriminated_unions
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `specta` | Add `specta::Type` derives for TypeScript codegen |
| `axum-handlers` | Generate axum server handler stubs and traits |

## Contributing

1. Fork the repo
2. Add your OpenAPI spec pattern to `examples/` or `tests/fixtures/`
3. Write a test using snapshot testing (`insta`)
4. Run `cargo insta test` and review the output
5. Submit a PR

## License

MIT