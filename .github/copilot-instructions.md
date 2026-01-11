# Blockscout Rust Services - AI Coding Guidelines

## Architecture Overview
This is a monorepo containing multiple microservices for the Blockscout blockchain explorer, written in Rust. Each service follows a consistent proto/logic/server pattern:

- `{service-name}-proto`: gRPC protocol definitions and generated code
- `{service-name}-logic`: Business logic implementation
- `{service-name}-server`: HTTP/gRPC server with transport layer
- `{service-name}-entity`: SeaORM database entities (when DB required)
- `{service-name}-migration`: Database migrations (when DB required)

Services communicate via gRPC internally and expose HTTP APIs externally. Data flows from Blockscout instances → services → PostgreSQL databases.

## Configuration Patterns
Environment variables use `{SERVICE_NAME}__` prefix with `__` separators:
```bash
SMART_CONTRACT_VERIFIER__SERVER__HTTP__ADDR=0.0.0.0:8050
ETH_BYTECODE_DB__DATABASE__URL=postgres://...
```

Common settings loaded via `blockscout_service_launcher::launcher::ConfigSettings`. See `docs/common-envs.md` for full reference.

## Development Workflow
- **Build**: `cargo build --release --bin {service-name}-server` (requires protoc >=3.15.0)
- **Format**: `cargo fmt --all -- --config imports_granularity=Crate`
- **Lint**: `cargo clippy --all --all-targets --all-features -- -D warnings`
- **Test**: `cargo test` (use justfiles for DB-dependent tests)
- **Run**: Docker preferred, or `cargo run --bin {service-name}-server`

Most services include `justfile` for common tasks like DB setup/migration.

## Code Patterns
- **Error Handling**: Use `thiserror` for custom errors, implement `From<T> for tonic::Status` for gRPC
- **Database**: SeaORM with transactions for atomic operations
- **Async**: Tokio runtime, prefer functional combinators over imperative loops
- **Logging**: `tracing` crate, instrument functions with `#[instrument]`
- **Immutability**: Default to immutable, mutable only when necessary

## Key Files to Understand
- `docs/common-envs.md`: Configuration reference
- `docs/build.md`: Build requirements and instructions
- `RUST_CODE_STYLE_GUIDE.md`: Detailed coding standards
- `README.md`: Service overview and project structure
- Individual service READMEs for service-specific details

## Integration Points
- **gRPC Services**: Defined in proto files, generated via `build.rs`
- **HTTP APIs**: Mapped from gRPC via `api_config_http.yaml`
- **Databases**: PostgreSQL with SeaORM entities/migrations
- **External APIs**: Sourcify, Etherscan, Verifier Alliance
- **Docker**: All services containerized with multi-stage builds

## Common Pitfalls
- Protoc version must be >=3.15.0 for proto3 optional fields
- Database URLs must include credentials for migrations
- CORS settings required for web client integration
- Functional style: avoid mutable state, prefer map/filter chains