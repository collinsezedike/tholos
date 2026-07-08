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

Import the client from the deployed contract's WASM and call it like any other
cross-contract invocation:

```rust
use soroban_sdk::{contractimport, Address, Env};

mod tholos {
    soroban_sdk::contractimport!(
        file = "../tholos/target/wasm32v1-none/release/tholos.wasm"
    );
}

fn resolve_my_condition(env: Env, tholos_id: Address, outcome: bool) -> u64 {
    let client = tholos::Client::new(&env, &tholos_id);
    client.assert_outcome(&env.current_contract_address(), &outcome)
}
```

Your contract's address can be the `asserter`, in which case your contract's own
`require_auth` logic (or lack of it) governs who can trigger an assertion on your
behalf. Whoever ends up as `asserter` is whoever gets the bond back on an
uncontested finalize, so decide deliberately whether that should be your contract
(pooling bonds under your own control) or an end user's address (bond returns
directly to them).

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
