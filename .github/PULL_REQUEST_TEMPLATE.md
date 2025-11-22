<!-- Pull Request template for Gemmy -->

# Pull Request Checklist

Use this checklist to keep PRs small, reviewable and consistent.

- [ ] **Build:** `cargo build --workspace --verbose` (this runs `build.rs` and regenerates protos).
  - If you changed `.proto` files: include the regenerated files from `src/protobuf` in the PR.
  - Run `git diff --exit-code` after `cargo build` to ensure no generated files are unstaged.

- [ ] **Tests:** `cargo test --workspace --verbose` (add or update unit/integration tests for behavioral changes).
  - Integration tests that require Kafka: run with `docker-compose up -d` then `cargo test --test integration_tests -- --nocapture`.

- [ ] **Formatting:** `cargo fmt -- --check` (run `cargo fmt` to fix issues before pushing).

- [ ] **Lints:** `cargo clippy --workspace -- -D warnings` (fix warnings or justify them in the PR description).

- [ ] **Protos:** If you updated `resources/protobuf/*.proto`, include regenerated `src/protobuf/*` and note the proto changes in the PR description.

- [ ] **Docs & README:** update `README.md` or inline docs if public APIs/usage changed.

- [ ] **Benchmarks:** do not include `cargo bench` results or large benchmark artifacts in PRs. Bench changes may be added in separate PRs and run on scheduled workflows.

- [ ] **Scope & Tests:** keep PR focused; large refactors should be split and include migration notes when public APIs change.

- [ ] **CI / Generated Files:** ensure CI passes; if CI fails due to generated files, regenerate locally and include them.

Describe changes briefly and include any migration steps, proto diffs, or external dependency changes in the PR description.
