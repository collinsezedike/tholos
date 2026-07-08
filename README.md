# Tholos

Bonded assertion and dispute oracle for resolving real world outcomes. Resolution infra for prediction markets and anything else that needs a trustworthy yes/no.

## Status

The assertion and dispute contract (`contracts/tholos`) is implemented, tested, and has been deployed and exercised on Stellar testnet. See [CONTRACT.md](CONTRACT.md) for the interface.

## Overview

Prediction markets and similar products eventually need to answer a hard question: who decides what actually happened? Existing approaches either rely on token holder votes that can be captured by large holders with a stake in the outcome, or on a centralized, regulated party acting as sole resolver.

Tholos is a bonded assertion and dispute contract: anyone can propose an outcome by posting a bond, and a challenge window gives others the chance to dispute it before it finalizes. It is designed to be standalone and composable, so any contract that needs a trustworthy resolution of a real world event can plug into it rather than building its own oracle logic.

## Development

Requires the Rust toolchain with the `wasm32v1-none` target, plus the [Stellar CLI](https://developers.stellar.org/docs/tools/cli/install-cli) for building and deploying the contract.

```sh
# Run unit tests
cargo test

# Check formatting and lints (same checks CI runs)
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# Build the optimized contract wasm
cd contracts/tholos && stellar contract build
```

To exercise a fresh deploy end-to-end against Stellar testnet (deploy, initialize, assert, dispute, resolve):

```sh
bash scripts/testnet-smoke.sh
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

TBD
