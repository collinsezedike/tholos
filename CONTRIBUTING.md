# Contributing

## Setup

- Rust toolchain (stable) with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli), for building and deploying the contract

## Before opening a PR

Run the same checks CI runs:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test
```

If you changed the contract's public interface (functions, types, errors), update
[CONTRACT.md](CONTRACT.md) to match.

If you changed contract behavior in a way that affects the deployed flow, consider
running `bash scripts/testnet-smoke.sh` against testnet before opening the PR.

## Commit messages

One-line, imperative, conventional-commit style: `feat: `, `fix: `, `docs: `, `test: `,
`ci: `, etc., followed by a concise summary. No comma-separated lists of unrelated
changes in a single message; split them into separate commits instead.

## Style

- No em dashes, en dashes, or `--`/`---` in code, comments, docs, or commit messages.
  Use colons, semicolons, parentheses, or a single hyphen instead.
- No references to AI tooling or generation in commits, comments, or docs.
