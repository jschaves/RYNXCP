# Project conventions

## Language

- All documentation and README files must be written in **English**.
- Code comments and commit-ready text are in English.
- The end-user web UI is currently in Spanish; keep UI copy consistent unless a
  localization task explicitly says otherwise.

## Stack

- Rust workspace (`crates/*`), edition 2021, stable toolchain.
- Control plane: `rynxd` (root, Unix socket) + `rynx-api` (Axum, unprivileged).
- Frontend: dependency-free SPA in `web/` served by the API.
- Debian/Ubuntu is the production target; Windows uses simulation mode.

## Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
powershell -ExecutionPolicy Bypass -File scripts/smoke.ps1
```

The smoke test runs the daemon and API in simulation mode, exercises every major
feature end to end, and cleans up after itself.

## Security rules

- Never introduce a shell (`sh -c`); always use explicit `argv`.
- Validate input in both the API and the daemon.
- Keep generated paths inside managed roots; use atomic writes.
- Every privileged action must be audited.
- Never log secrets, tokens, passwords or private keys.
