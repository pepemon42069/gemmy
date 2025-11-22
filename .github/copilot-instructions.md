<!--
Guidance for AI coding agents working on the Gemmy repo.
Keep this concise and focused on repository-specific patterns, workflows, and files.
-->

# Copilot instructions for Gemmy

Short, actionable notes to help an AI coding agent be immediately productive in this Rust project.

- **Project shape:** This repo exposes a library crate (`lib.rs`) and an executable `gemmy-engine` (`src/main.rs`). The library modules live under `src/core` (models, orderbook, store) while `src/engine` contains runtime components (configuration, services, tasks, state, utils).

- **Where the business logic lives:** `src/core/orderbook.rs`, `src/core/models.rs` and `src/core/store.rs` implement the orderbook and domain models. Treat `core` as the canonical domain API — changes here affect both library consumers and the engine executable.

- **Runtime / orchestration:** `src/engine` wires the runtime: configuration loaders (`engine/configuration`), async services (`engine/services`), long-running tasks (`engine/tasks`), and process state (`engine/state`). When adding new runtime features, register them via `task_manager.rs` and service modules.

- **Protobuf / gRPC:** Protos live in `resources/protobuf/*.proto`. `build.rs` runs `tonic-build` and generates Rust code into `src/protobuf` (server only: `build_server(true)` and `build_client(false)`). Do NOT hand-edit generated files in `src/protobuf`; instead modify the .proto and run `cargo build` to regenerate.

- **Data flow summary:** Orders flow from ingestion services (`engine/services/*`) → `order_dispatch_service` → `orderbook_manager_service` → core orderbook logic (`core/orderbook.rs`). Stats/streams are in `stat_stream_service.rs`. Persistence/serialization is handled in `core/store.rs` and protobuf modules.

- **Kafka & schema registry:** Kafka integration appears in `engine/configuration/kafka_configuration.rs` and services use `rdkafka` + `schema_registry_converter`. Be careful with schema handling — the project uses `schema_registry_converter` with `proto_raw` feature; check `Cargo.toml` if changing proto or schema interactions.

- **Build / common commands:**
  - Build and regenerate protos: `cargo build` (runs `build.rs`)
  - Run the engine binary: `cargo run --bin gemmy-engine -- [args]`
  - Run tests: `cargo test` (integration tests live in `tests/integration_tests.rs`)
  - Run benchmarks (criterion): `cargo bench` or `cargo bench --bench gemmy_benchmarks` (uses `resources/orders.csv`)
  - Format / lint: `cargo fmt` and `cargo clippy` (recommended before PRs)

- **Docker / compose:** There is a `Dockerfile` and `docker-compose.yml` for containerized runs. Use them when testing integrations that require Kafka or other services.

- **Benchmarks & sample data:** Bench source is `benches/gemmy_benchmarks.rs`. Sample order CSVs are in `resources/orders.csv` and `target/criterion` contains previous benchmark outputs — helpful for reasoning about perf expectations.

- **Testing conventions:** Unit tests are colocated in modules. Integration tests are in `tests/`. When modifying core behaviour, update both unit tests and the integration test in `tests/integration_tests.rs` to avoid regressions.

- **Code and PR guidance for agents:**
  - Small, focused PRs: change one domain behaviour or one engine integration at a time.
  - If you modify protos (`resources/protobuf/*.proto`), run `cargo build` locally to regenerate `src/protobuf/*` and include generated changes in the PR.
  - When adding services/tasks, wire them through `engine/tasks/task_manager.rs` and register logging in `engine/configuration/log_configuration.rs`.

- **Files to check for examples or patterns:**
  - Domain: `src/core/orderbook.rs`, `src/core/models.rs`, `src/core/store.rs`
  - Runtime: `src/engine/services/*`, `src/engine/tasks/*`, `src/engine/state/server_state.rs`
  - Protobuf gen: `build.rs`, `resources/protobuf/*.proto`, `src/protobuf/*`
  - Benchmarks: `benches/gemmy_benchmarks.rs`, `resources/orders.csv`

- **What not to do:**
  - Do not manually hand-edit files under `src/protobuf` as they are generated from protos.
  - Avoid changing crate public APIs in large sweeps without updating `README.md` examples and tests.

- **Quick note about `build.rs`:** it intentionally sets `build_client(false)` — the repo generates server-side stubs only. If a change requires client generation, update `build.rs` accordingly and regenerate.

If anything here is incomplete or you want the agent to follow stricter rules (style, commit message format, or CI details), tell me what to add and I will iterate.

**CI Notes**

- **Primary goals:** Ensure `cargo build`, `cargo test`, formatting (`cargo fmt -- --check`), and linting (`cargo clippy`) run on PRs; catch unexpected proto-generated changes and avoid slow/optional benchmarks in the default PR run.

- **Suggested pipeline steps (PR):**
  - `actions/checkout`
  - `actions-rs/toolchain` to set up the Rust toolchain (use `stable`)
  - cache `~/.cargo/registry` and `target` using `actions/cache`
  - `cargo build --workspace --verbose` (this runs `build.rs` and regenerates protos)
  - run `git diff --exit-code` to fail if `build.rs` generated changes not included in the PR
  - `cargo test --workspace --verbose`
  - `cargo fmt -- --check`
  - `cargo clippy --workspace -- -D warnings`

- **Protobuf tip:** `build.rs` runs `tonic-build` and writes generated code to `src/protobuf`. CI must run `cargo build` and verify no unstaged changes. If PR changes `.proto` files, include the regenerated files from `src/protobuf` in the PR.

- **Integration tests (Kafka):** Integration tests may require Kafka/Zookeeper. For CI jobs that run integration tests, bring up the stack with `docker-compose.yml` before running the tests:

```bash
docker-compose up -d
cargo test --test integration_tests -- --nocapture
docker-compose down
```

- **Benchmarks:** `cargo bench` is expensive — run separately (nightly or manual). If you add a benchmark job, mark it as optional or scheduled.

- **Example minimal GitHub Actions job (PR):**

```yaml
name: CI
on: [pull_request]
jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Build (regen protos)
        run: cargo build --workspace --verbose
      - name: Fail if generated files changed
        run: git diff --exit-code
      - name: Tests
        run: cargo test --workspace --verbose
      - name: Format check
        run: cargo fmt -- --check
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings
```

If you want, I can also add a ready-to-commit `.github/workflows/ci.yml` file based on the above snippet.
