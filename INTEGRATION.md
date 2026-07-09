# Integrating with Tholos

For contracts that need a trustworthy resolution of a real world outcome and want
to call into Tholos rather than build their own propose/dispute/resolve logic. If
you're looking for the function-by-function reference instead, see
[CONTRACT.md](CONTRACT.md).

## Should you deploy your own instance, or share one?

Each Tholos deployment is initialized once with a single token, bond amount,
challenge window, and resolver committee (`initialize` in [CONTRACT.md](CONTRACT.md)).
There's no per-call override. That means:

- If your markets all want the same bond size, token, and challenge window, they can
  share one deployed instance and just track the assertion `id`s that belong to them.
- If you need different bond sizes per market (a $10 market and a $10,000 market
  probably shouldn't share a bond amount), deploy a separate instance per
  configuration, or wait for a future version that supports per-call bonds.

There is currently no built-in way for a calling contract to distinguish "its"
assertions from anyone else's within one instance beyond tracking the `id`s it
received back from `assert_outcome`. Store that mapping on your side (e.g.
`market_id -> assertion_id`).

## Calling Tholos from another Soroban contract

`contracts/demo-consumer` is a working, tested example of this, not just a
snippet: its `create_assertion` and `get_status` functions are the pattern below,
and its test deploys Tholos's actual compiled wasm and calls through it. If
anything here goes stale, that crate's `cargo test -p demo-consumer` is what
would catch it.

Import the client from the deployed contract's WASM and call it like any other
cross-contract invocation:

```rust
use soroban_sdk::{contractimport, Address, Env};

mod tholos {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/tholos.wasm"
    );
}

fn create_assertion(env: Env, tholos_id: Address, asserter: Address, outcome: bool) -> u64 {
    let client = tholos::Client::new(&env, &tholos_id);
    client.assert_outcome(&asserter, &outcome)
}
```

`contractimport!` reads the wasm file **at your crate's compile time**, so it has
to already exist on disk before you build. In this repo that means running
`cargo build -p tholos --target wasm32v1-none --release` before touching
`demo-consumer` (see [CONTRIBUTING.md](CONTRIBUTING.md)); if Tholos is a separate
repo for you, the same constraint applies to wherever its wasm gets built.

### Who should be the `asserter`: your contract, or the end user?

This is the decision that has the most integration friction, and it's worth
getting right before you write the code.

**End user as asserter (what `demo-consumer` does, and the default recommendation).**
Pass through an `Address` the caller provides, as above. The user's own signature
authorizes `assert_outcome` and the underlying bond transfer directly; your
contract doesn't need any special auth plumbing. The tradeoff: because that
signature lives on an argument to *your* function rather than the top-level call,
if you're writing tests against this you need
`env.mock_all_auths_allowing_non_root_auth()` rather than plain `mock_all_auths()`
(see `demo-consumer/src/test.rs`), and on a real network the transaction needs an
authorization entry for that address alongside whatever signs the outer call.

**Your contract's own address as asserter.** Bonds pool under your contract's
control (e.g. to later distribute pro-rata to your own users) instead of going
directly to an end user. This is meaningfully harder than it looks: Tholos's
`assert_outcome` calls the underlying token's `transfer`, which itself calls
`require_auth()` on the asserter. That's *two* contract calls away from your
contract (yours -> Tholos -> token), and Soroban only auto-grants a contract's
implicit self-authorization one call deep. The deeper call fails with
`Error(Auth, InvalidAction)` unless you explicitly pre-authorize it with
[`env.authorize_as_current_contract`](https://docs.rs/soroban-sdk/latest/soroban_sdk/struct.Env.html#method.authorize_as_current_contract)
before invoking Tholos, specifying the exact token contract, `transfer` args, and
amount Tholos will end up calling. That means you need to already know Tholos's
configured token and bond amount to construct the right authorization, since
there's no way to ask Tholos for the sub-invocation it's about to make ahead of
time. Only take this path if pooling bonds under your contract is a real
requirement, not a default choice.

## Lifecycle from an integrator's perspective

`finalize` and `resolve` are both permissionless: anyone (a keeper, a bot, an end
user, your own contract) can call them once the preconditions are met. Tholos does
not push a callback to your contract when an assertion resolves. If you need to
react automatically, two options:

1. **Poll** `get_assertion_state(id)` after the challenge window you configured has
   elapsed, and act once `status` is `Resolved`.
2. **Watch events.** Every state transition emits an event (see the table in
   [CONTRACT.md](CONTRACT.md#events)); an off-chain indexer or keeper watching
   `Finalized`/`Resolved` for your tracked `id`s can call back into your contract
   once the outcome is final.

Either way, build your integration assuming resolution is not instant: it takes at
least the full challenge window, and longer if disputed and resolver votes trickle
in slowly.

## Reading the outcome

```rust
let state = client.get_assertion_state(&id);
match state.status {
    tholos::Status::Resolved => {
        // state.outcome reflects the *original* asserted outcome, not necessarily
        // the final one if the assertion was disputed and overturned. Prefer the
        // Finalized/Resolved event payload (`outcome` field), which is always the
        // final decided outcome, over re-deriving it from Assertion.outcome.
    }
    _ => { /* not resolved yet */ }
}
```

This is a sharp edge worth calling out explicitly: `Assertion.outcome` is the
*claimed* outcome at assertion time and is not flipped in storage if a dispute
overturns it. The authoritative final outcome is what the `Finalized` or `Resolved`
event carries, not `get_assertion_state(id).outcome`.

## Parameters you're choosing when you initialize

| Parameter | Consideration |
| --- | --- |
| `token` | Any SEP-41 token. Must be a token your users already hold or can acquire; bonds are paid in it directly, there's no swap step. |
| `bond_amount` | High enough to deter spam/bad-faith assertions, low enough that legitimate use isn't priced out. Fixed per instance, see above. |
| `challenge_window_secs` | Longer windows give more time to catch bad assertions but delay uncontested finalization. |
| `resolvers` | Must be odd-length. See [CONTRACT.md](CONTRACT.md) for what `update_resolvers` can and can't change mid-dispute. |

## Known caveats for integrators

- No reward beyond bond-return for uncontested finalizes: there's currently no fee
  mechanism, so integrators who want to incentivize keepers to call `finalize`
  promptly need to handle that themselves (e.g. your own contract pays a small
  bounty to whoever triggers your callback).
- No pause: if something goes wrong, there's no way to freeze in-flight assertions
  short of `update_resolvers` locking out the compromised committee.
