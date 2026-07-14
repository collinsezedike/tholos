#![cfg(test)]

use super::*;
use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::{Address as _, Ledger};

const DEFAULT_BOND: i128 = 100;
const DEFAULT_WINDOW: u64 = 3600;
const DEFAULT_MINT: i128 = 1_000;

/// A registered but uninitialized token and resolver committee, for the
/// handful of tests that need to call `initialize` themselves (to test bad
/// init parameters, or that it can't be called twice).
fn setup(env: &Env) -> (Address, Vec<Address>) {
    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let resolvers = Vec::from_array(
        env,
        [
            Address::generate(env),
            Address::generate(env),
            Address::generate(env),
        ],
    );
    (token_id, resolvers)
}

/// A ready-to-use, already-initialized Tholos instance with a 3-member
/// resolver committee and its backing token (bond 100, window 3600), used by
/// most tests. Tests that need an *uninitialized* contract, or non-default
/// init parameters, use `setup` directly instead.
struct Fixture {
    env: Env,
    client: TholosClient<'static>,
    token: token::Client<'static>,
    token_id: Address,
    resolvers: Vec<Address>,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let (token_id, resolvers) = setup(&env);
        let token = token::Client::new(&env, &token_id);

        let contract_id = env.register(Tholos, ());
        let client = TholosClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(
            &admin,
            &token_id,
            &DEFAULT_BOND,
            &DEFAULT_WINDOW,
            &resolvers,
        );

        Fixture {
            env,
            client,
            token,
            token_id,
            resolvers,
        }
    }

    fn generate(&self) -> Address {
        Address::generate(&self.env)
    }

    /// Generates a fresh address and mints it the default test balance.
    fn funded_address(&self) -> Address {
        let addr = self.generate();
        self.mint(&addr, DEFAULT_MINT);
        addr
    }

    fn mint(&self, addr: &Address, amount: i128) {
        token::StellarAssetClient::new(&self.env, &self.token_id).mint(addr, &amount);
    }

    fn advance_past_window(&self) {
        self.env
            .ledger()
            .with_mut(|l| l.timestamp += DEFAULT_WINDOW + 1);
    }
}

#[test]
fn test_uncontested_assertion_finalizes() {
    let f = Fixture::new();
    let asserter = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    assert_eq!(f.token.balance(&asserter), 900);

    f.advance_past_window();

    let outcome = f.client.finalize(&id);
    assert!(outcome);
    assert_eq!(f.token.balance(&asserter), 1_000);
}

#[test]
fn test_assertion_storage_ttl_is_extended_on_every_write() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let ttl_of = |id: u64| {
        f.env.as_contract(&f.client.address, || {
            f.env
                .storage()
                .persistent()
                .get_ttl(&DataKey::Assertion(id))
        })
    };

    let id = f.client.assert_outcome(&asserter, &true);
    assert_eq!(ttl_of(id), ASSERTION_BUMP_AMOUNT);

    // Advance close to expiry, then confirm disputing (a write) bumps the
    // TTL back up rather than leaving the entry to lapse.
    f.env
        .ledger()
        .with_mut(|l| l.sequence_number += ASSERTION_BUMP_AMOUNT - 10);
    f.client.dispute(&disputer, &id);
    assert_eq!(ttl_of(id), ASSERTION_BUMP_AMOUNT);
}

#[test]
fn test_disputed_assertion_pays_winner() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);
    assert_eq!(f.token.balance(&asserter), 900);
    assert_eq!(f.token.balance(&disputer), 900);

    f.client.resolve(&f.resolvers.get(0).unwrap(), &id, &false);
    f.client.resolve(&f.resolvers.get(1).unwrap(), &id, &false);

    assert_eq!(f.token.balance(&disputer), 1_100);
    assert_eq!(f.token.balance(&asserter), 900);
}

#[test]
fn test_cannot_initialize_with_even_resolver_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_id, _resolvers) = setup(&env);
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let even_resolvers = Vec::from_array(&env, [Address::generate(&env), Address::generate(&env)]);

    let result = client.try_initialize(
        &admin,
        &token_id,
        &DEFAULT_BOND,
        &DEFAULT_WINDOW,
        &even_resolvers,
    );
    assert!(result.is_err());
}

#[test]
fn test_cannot_initialize_with_zero_bond_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_id, resolvers) = setup(&env);
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let result = client.try_initialize(&admin, &token_id, &0, &DEFAULT_WINDOW, &resolvers);
    assert_eq!(result, Err(Ok(Error::InvalidBondAmount)));
}

#[test]
fn test_cannot_initialize_with_negative_bond_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_id, resolvers) = setup(&env);
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let result = client.try_initialize(&admin, &token_id, &-1, &DEFAULT_WINDOW, &resolvers);
    assert_eq!(result, Err(Ok(Error::InvalidBondAmount)));
}

#[test]
fn test_cannot_initialize_with_zero_challenge_window() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_id, resolvers) = setup(&env);
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let result = client.try_initialize(&admin, &token_id, &DEFAULT_BOND, &0, &resolvers);
    assert_eq!(result, Err(Ok(Error::InvalidChallengeWindow)));
}

#[test]
fn test_cannot_initialize_with_challenge_window_too_large() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_id, resolvers) = setup(&env);
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let result = client.try_initialize(
        &admin,
        &token_id,
        &DEFAULT_BOND,
        &(MAX_CHALLENGE_WINDOW_SECS + 1),
        &resolvers,
    );
    assert_eq!(result, Err(Ok(Error::InvalidChallengeWindow)));
}

#[test]
fn test_cannot_initialize_twice() {
    let f = Fixture::new();

    let admin = f.generate();
    let result = f.client.try_initialize(
        &admin,
        &f.token_id,
        &DEFAULT_BOND,
        &DEFAULT_WINDOW,
        &f.resolvers,
    );
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_cannot_finalize_before_window_closes() {
    let f = Fixture::new();
    let asserter = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);

    let result = f.client.try_finalize(&id);
    assert_eq!(result, Err(Ok(Error::ChallengeWindowOpen)));
}

#[test]
fn test_cannot_dispute_after_window_closes() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    f.advance_past_window();

    let result = f.client.try_dispute(&disputer, &id);
    assert_eq!(result, Err(Ok(Error::ChallengeWindowClosed)));
}

#[test]
fn test_cannot_dispute_an_already_disputed_assertion() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();
    let second_disputer = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);

    let result = f.client.try_dispute(&second_disputer, &id);
    assert_eq!(result, Err(Ok(Error::NotPending)));
}

#[test]
fn test_non_resolver_cannot_vote() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();
    let outsider = f.generate();

    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);

    let result = f.client.try_resolve(&outsider, &id, &true);
    assert_eq!(result, Err(Ok(Error::NotAResolver)));
}

#[test]
fn test_resolver_cannot_vote_twice() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);

    let resolver = f.resolvers.get(0).unwrap();
    f.client.resolve(&resolver, &id, &true);

    let result = f.client.try_resolve(&resolver, &id, &true);
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

#[test]
fn test_cannot_resolve_a_non_disputed_assertion() {
    let f = Fixture::new();
    let asserter = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);

    let result = f
        .client
        .try_resolve(&f.resolvers.get(0).unwrap(), &id, &true);
    assert_eq!(result, Err(Ok(Error::NotDisputed)));
}

#[test]
fn test_split_resolver_vote_does_not_finalize() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);

    let outcome = f.client.resolve(&f.resolvers.get(0).unwrap(), &id, &true);
    assert_eq!(outcome, None);
    assert_eq!(f.token.balance(&asserter), 900);
    assert_eq!(f.token.balance(&disputer), 900);
}

#[test]
fn test_admin_can_update_resolvers() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    let new_resolvers = Vec::from_array(&f.env, [f.generate(), f.generate(), f.generate()]);
    f.client.update_resolvers(&new_resolvers);

    // The old committee can no longer vote.
    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);
    let result = f
        .client
        .try_resolve(&f.resolvers.get(0).unwrap(), &id, &true);
    assert_eq!(result, Err(Ok(Error::NotAResolver)));

    // The new committee can.
    f.client
        .resolve(&new_resolvers.get(0).unwrap(), &id, &false);
    f.client
        .resolve(&new_resolvers.get(1).unwrap(), &id, &false);
    assert_eq!(f.token.balance(&disputer), 1_100);
}

#[test]
fn test_resolvers_updated_mid_dispute_do_not_affect_it() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    // The dispute is opened, snapshotting the original committee, before the
    // committee changes.
    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);

    let new_resolvers = Vec::from_array(&f.env, [f.generate(), f.generate(), f.generate()]);
    f.client.update_resolvers(&new_resolvers);

    // A member of the new (current) committee cannot vote on this dispute:
    // it was snapshotted to the old committee before they joined.
    assert_eq!(
        f.client
            .try_resolve(&new_resolvers.get(0).unwrap(), &id, &true),
        Err(Ok(Error::NotAResolver))
    );

    // The original committee, though no longer the live committee, can
    // still decide this dispute, since it was snapshotted at dispute time.
    f.client.resolve(&f.resolvers.get(0).unwrap(), &id, &false);
    f.client.resolve(&f.resolvers.get(1).unwrap(), &id, &false);
    assert_eq!(f.token.balance(&disputer), 1_100);
}

#[test]
fn test_paused_blocks_assert_dispute_and_resolve_but_not_finalize() {
    let f = Fixture::new();
    let asserter = f.funded_address();
    let disputer = f.funded_address();

    // An assertion posted before the pause can still finalize normally.
    let pending_id = f.client.assert_outcome(&asserter, &true);

    f.client.set_paused(&true);

    assert_eq!(
        f.client.try_assert_outcome(&asserter, &true),
        Err(Ok(Error::Paused))
    );
    assert_eq!(
        f.client.try_dispute(&disputer, &pending_id),
        Err(Ok(Error::Paused))
    );

    f.advance_past_window();
    let outcome = f.client.finalize(&pending_id);
    assert!(outcome);
    assert_eq!(f.token.balance(&asserter), 1_000);

    f.client.set_paused(&false);
    let id = f.client.assert_outcome(&asserter, &true);
    f.client.dispute(&disputer, &id);
    assert_eq!(
        f.client
            .try_resolve(&f.resolvers.get(0).unwrap(), &id, &true),
        Ok(Ok(None))
    );
}

#[test]
fn test_cannot_pause_before_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    assert_eq!(client.try_set_paused(&true), Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_cannot_update_resolvers_to_even_count() {
    let f = Fixture::new();

    let even_resolvers = Vec::from_array(&f.env, [f.generate(), f.generate()]);
    let result = f.client.try_update_resolvers(&even_resolvers);
    assert_eq!(result, Err(Ok(Error::InvalidResolverCount)));
}

#[test]
fn test_cannot_update_resolvers_before_initialization() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let resolvers = Vec::from_array(
        &env,
        [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ],
    );
    let result = client.try_update_resolvers(&resolvers);
    assert_eq!(result, Err(Ok(Error::NotInitialized)));
}

#[test]
fn test_operations_on_unknown_assertion_fail() {
    let f = Fixture::new();
    let disputer = f.generate();

    assert_eq!(
        f.client.try_dispute(&disputer, &42),
        Err(Ok(Error::AssertionNotFound))
    );
    assert_eq!(
        f.client.try_finalize(&42),
        Err(Ok(Error::AssertionNotFound))
    );
    assert_eq!(
        f.client
            .try_resolve(&f.resolvers.get(0).unwrap(), &42, &true),
        Err(Ok(Error::AssertionNotFound))
    );
    assert_eq!(
        f.client.try_get_assertion_state(&42),
        Err(Ok(Error::AssertionNotFound))
    );
}

/// A minimal token that reenters Tholos's `finalize` from inside its own
/// `transfer`, before doing its own balance bookkeeping. Models a malicious
/// or merely non-standard (e.g. hook-bearing) SEP-41 token, to prove state is
/// written before the external transfer rather than after it.
mod evil_token {
    use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Map};

    #[contract]
    pub struct EvilToken;

    #[contractimpl]
    impl EvilToken {
        pub fn configure(env: Env, tholos_id: Address, target_id: u64) {
            env.storage()
                .instance()
                .set(&symbol_short!("tholos"), &tholos_id);
            env.storage()
                .instance()
                .set(&symbol_short!("target"), &target_id);
        }

        pub fn credit(env: Env, addr: Address, amount: i128) {
            let mut balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or(Map::new(&env));
            let current = balances.get(addr.clone()).unwrap_or(0);
            balances.set(addr, current + amount);
            env.storage()
                .instance()
                .set(&symbol_short!("bal"), &balances);
        }

        pub fn balance(env: Env, addr: Address) -> i128 {
            let balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or(Map::new(&env));
            balances.get(addr).unwrap_or(0)
        }

        pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
            if let Some(tholos_id) = env
                .storage()
                .instance()
                .get::<_, Address>(&symbol_short!("tholos"))
            {
                let target_id: u64 = env
                    .storage()
                    .instance()
                    .get(&symbol_short!("target"))
                    .unwrap();
                let client = super::TholosClient::new(&env, &tholos_id);
                // A well-behaved caller would fail cleanly here if Tholos has
                // already written its state; that's exactly what this test
                // verifies. Ignore the result either way.
                let _ = client.try_finalize(&target_id);
            }

            let mut balances: Map<Address, i128> = env
                .storage()
                .instance()
                .get(&symbol_short!("bal"))
                .unwrap_or(Map::new(&env));
            let from_bal = balances.get(from.clone()).unwrap_or(0);
            let to_bal = balances.get(to.clone()).unwrap_or(0);
            balances.set(from, from_bal - amount);
            balances.set(to, to_bal + amount);
            env.storage()
                .instance()
                .set(&symbol_short!("bal"), &balances);
        }
    }
}

#[test]
fn test_finalize_is_not_reentrant() {
    use evil_token::{EvilToken, EvilTokenClient};

    let env = Env::default();
    env.mock_all_auths();

    let evil_token_id = env.register(EvilToken, ());
    let evil_token = EvilTokenClient::new(&env, &evil_token_id);

    let resolvers = Vec::from_array(
        &env,
        [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ],
    );
    let contract_id = env.register(Tholos, ());
    let client = TholosClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let asserter = Address::generate(&env);
    evil_token.credit(&asserter, &1_000);
    client.initialize(
        &admin,
        &evil_token_id,
        &DEFAULT_BOND,
        &DEFAULT_WINDOW,
        &resolvers,
    );

    // The reentrancy trap isn't armed yet, so this assert_outcome call's own
    // transfer doesn't try to reenter anything.
    let id = client.assert_outcome(&asserter, &true);
    assert_eq!(evil_token.balance(&asserter), 900);

    env.ledger().with_mut(|l| l.timestamp += DEFAULT_WINDOW + 1);

    // Arm the trap: EvilToken.transfer will now try to reenter finalize(id)
    // on itself, before finalize's own transfer call even returns.
    evil_token.configure(&contract_id, &id);

    let outcome = client.finalize(&id);
    assert!(outcome);

    // Exactly one bond's worth was returned, not two. If Tholos wrote state
    // after the transfer instead of before, the reentrant finalize call
    // would have seen the assertion as still `Pending` and paid out again.
    assert_eq!(evil_token.balance(&asserter), 1_000);
}
