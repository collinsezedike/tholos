# Contributing

## Setup

- Rust toolchain (stable) with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli), for building and deploying the contract

Clone the repo and confirm the toolchain is wired up:

```sh
cargo test
```

That runs the unit test suite against the native target (fast, no wasm build or
network access required) and is the right first thing to run after any change.

## Project layout

```text
contracts/
  tholos/            The assertion and dispute contract
    src/
      lib.rs          Contract logic
      test.rs         Unit tests (soroban-sdk testutils, mocked ledger and auth)
scripts/
  testnet-smoke.sh    End-to-end check against real Stellar testnet infrastructure
.github/workflows/
  ci.yml              Runs fmt, clippy, tests, and the wasm build on every push/PR
```

If a second contract is added later (e.g. a market factory), it should live as its
own crate under `contracts/`, added to the `[workspace] members` list in the root
`Cargo.toml`, following the same layout as `contracts/tholos`.

## Testing philosophy

There are two layers, and they catch different things:

- **Unit tests** (`cargo test`) run against a mocked ledger and mocked auth. Fast,
  deterministic, and where most new behavior should be covered, including every new
  `Error` variant you introduce: if you add a new failure path, add a test that
  triggers it.
- **The testnet smoke script** (`scripts/testnet-smoke.sh`) deploys to a real
  network and exercises real auth, real storage TTLs, and a real SAC token. This is
  the only thing that can catch a class of bug unit tests structurally can't (for
  example, an auth check that's satisfied by `mock_all_auths()` in tests but fails
  against a real signature). Run it before opening a PR that changes contract
  behavior in a way that affects the deployed flow, not for every change.

## Before opening a PR

Run the same checks CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

If you changed the contract's public interface (functions, types, errors), update
[CONTRACT.md](CONTRACT.md) to match; it's meant to stay in sync with `lib.rs`, not
drift into a separate design doc.

## Commit messages

One-line, imperative, conventional-commit style: `feat: `, `fix: `, `docs: `, `test: `,
`ci: `, etc., followed by a concise summary. No comma-separated lists of unrelated
changes in a single message; split them into separate commits instead.

## Opening a PR

CI (fmt, clippy, tests, wasm build) must pass before merge. Describe what changed and
why, not just what: if the change affects bond amounts, resolver behavior, or
anything with an economic consequence, say so explicitly in the description so it's
easy to reason about from the PR alone.
