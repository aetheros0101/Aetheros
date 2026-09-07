# AetherOS Hardened — Security Fix Pass

## Fixed in this pass

- JWT signing changed from `SHA256(secret || message)` to RFC 2104 HMAC-SHA256.
- JWT encoding changed to real unpadded base64url and the decoded header is checked for `HS256` + `JWT`.
- Predictable `dev-secret-change-in-production` fallback removed; missing JWT secret now gets a process-random secret and a high-severity log. Production should still set `AETHEROS_JWT_SECRET`.
- `ApiKey` no longer keeps the raw API key in memory.
- WASM module size, count and total in-memory store limits added; limits are configurable with environment variables.
- REST request-size/resource validation added for task, agent, workflow and script APIs.
- Wasmtime store memory/instance/table limits are now actually enforced with `StoreLimits`.
- AES-256-GCM at-rest encryption implemented with `ring` when `AETHEROS_ENCRYPTION_KEY` is configured.
- `cargo audit` made blocking in CI.
- Android Rust/Flutter build commands no longer hide non-zero exit status with `grep ... || true`, and old `.so` files are removed before a build.
- Duplicate `State<AppState>` argument in the workflow handler was removed.
- JWT regression tests were added.

## Not fully resolved in this pass

- The Wasmi 0.31 backend still uses its existing timeout-based execution path. A timeout around `spawn_blocking` does not forcibly terminate a native thread. A safe deterministic fuel-based implementation requires upgrading the Wasmi dependency/API or moving execution into a killable worker process.
- `/remote/command` remains a protocol stub for ExecuteTask/CancelExecution; it does not yet dispatch/cancel the local runtime.
- REST authorization is still route-group based rather than per-`Action` RBAC for every endpoint.
- Cluster transport still needs real TLS/mTLS; the shared cluster token only authenticates the peer.
- Existing plaintext sled databases need a migration before enabling AES-GCM encryption with a key.

## Validation

The provided environment does not contain `cargo`/`rustc`, so `cargo fmt`, `cargo check`, `cargo test` and `cargo clippy` could not be executed here. The archive is therefore a source-level security fix pass and should be compiled/tested in a Rust toolchain before production deployment.
