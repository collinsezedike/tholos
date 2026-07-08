# Contract interface

Reference for `contracts/tholos`. Source of truth is `contracts/tholos/src/lib.rs`; this
document should be updated alongside any change to the public interface.

## Types

### `Status`

State of an assertion: `Pending`, `Disputed`, or `Resolved`.

### `Assertion`

| Field | Type | Meaning |
| --- | --- | --- |
| `asserter` | `Address` | Who posted the claim |
| `outcome` | `bool` | The claimed outcome |
| `bond` | `i128` | Bond amount posted (in the configured token) |
| `opened_at` | `u64` | Ledger timestamp the assertion was posted |
| `status` | `Status` | Current state |
| `disputer` | `Option<Address>` | Who disputed it, if disputed |
| `votes_for_outcome` / `votes_against_outcome` | `u32` | Resolver vote tally |
| `voted` | `Vec<Address>` | Resolvers who have already voted, to prevent double-voting |

### `Error`

| Variant | Meaning |
| --- | --- |
| `AlreadyInitialized` | `initialize` called on a contract that's already set up |
| `NotInitialized` | Called before `initialize` (e.g. `update_resolvers`) |
| `InvalidResolverCount` | Resolver list is empty or has an even length |
| `AssertionNotFound` | No assertion exists with the given id |
| `NotPending` | Action requires `Status::Pending` but the assertion isn't |
| `NotDisputed` | Action requires `Status::Disputed` but the assertion isn't |
| `ChallengeWindowClosed` | Tried to dispute after the challenge window elapsed |
| `ChallengeWindowOpen` | Tried to finalize before the challenge window elapsed |
| `NotAResolver` | Caller isn't in the current resolver committee |
| `AlreadyVoted` | Resolver already voted on this assertion |

## Functions

### `initialize(admin, token, bond_amount, challenge_window_secs, resolvers)`

One-time setup. `resolvers` must have an odd, non-zero length so a majority vote can
never tie. Requires `admin`'s signature. Fails with `AlreadyInitialized` if called
twice.

### `update_resolvers(new_resolvers)`

Replaces the resolver committee. Requires the stored admin's signature. Same
odd-length requirement as `initialize`. Emits `ResolversUpdated`. Does not affect the
`voted` list on in-flight assertions: a resolver removed mid-dispute simply can no
longer cast further votes; a resolver added mid-dispute can vote on assertions that
were already disputed before they joined.

### `assert_outcome(asserter, outcome) -> u64`

Posts a bonded claim. Transfers `bond_amount` from `asserter` to the contract.
Requires `asserter`'s signature. Returns the new assertion id. Emits `Asserted`.

### `dispute(disputer, id)`

Disputes a `Pending` assertion within the challenge window, matching its bond.
Requires `disputer`'s signature. Fails with `NotPending` if the assertion isn't
pending (including if it's already disputed), or `ChallengeWindowClosed` if the
window has elapsed. Emits `Disputed`.

### `finalize(id) -> bool`

Callable by anyone once a `Pending` assertion's challenge window has elapsed with no
dispute. Returns the asserter's bond and returns the asserted outcome. Fails with
`ChallengeWindowOpen` if called too early. Emits `Finalized`.

### `resolve(resolver, id, agrees_with_asserter) -> Option<bool>`

Casts one resolver's vote on a `Disputed` assertion. Requires `resolver`'s signature
and that they're in the current committee. Fails with `NotAResolver`,
`NotDisputed`, or `AlreadyVoted` as appropriate.

Returns `None` if no side has reached a strict majority yet. Once a majority agrees,
the winning side (asserter if the majority agreed with them, disputer otherwise)
receives both bonds, the assertion moves to `Resolved`, a `Resolved` event is
emitted, and the function returns `Some(final_outcome)`.

### `get_assertion_state(id) -> Assertion`

Read-only lookup. Fails with `AssertionNotFound` if the id doesn't exist.

## Known gaps

- No fee/reward mechanism for uncontested finalizes: the original design called for
  a small reward funded by market fees, but no fee-generating market layer exists
  yet, so `finalize` just returns the bond as-is.
- No pause or emergency-stop.
- `update_resolvers` is a single-admin-key operation, which is a bigger centralization
  point than the resolver committee itself. A resolver self-rotation scheme (the
  committee votes to replace one of its own) was considered but not built for v1.
