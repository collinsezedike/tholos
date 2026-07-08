## Summary

What changed and why, not just a list of files touched.

## Test plan

- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test` pass locally
- [ ] `CONTRACT.md` updated if the public interface changed
- [ ] `scripts/testnet-smoke.sh` run against testnet, if this changes contract behavior in a way that affects the deployed flow
- [ ] What you manually verified
