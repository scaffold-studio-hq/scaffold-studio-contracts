// Integration tests for the Stellar Studio factory contracts.
//
// These tests exercise the real, cross-contract deployment path end to end. The
// contract WASM artifacts are embedded with `contractimport!`, so build them
// first from the workspace root:
//
//     stellar contract build
//     cargo test --test integration_tests

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env, IntoVal, String, Symbol, Val,
};

mod master_factory_wasm {
    soroban_sdk::contractimport!(file = "target/wasm32v1-none/release/master_factory.wasm");
}

mod token_factory_wasm {
    soroban_sdk::contractimport!(file = "target/wasm32v1-none/release/token_factory.wasm");
}

mod fungible_allowlist_wasm {
    soroban_sdk::contractimport!(file = "target/wasm32v1-none/release/fungible_allowlist_example.wasm");
}

/// Deploys a MasterFactory from its built WASM and returns its address.
fn deploy_master_factory(env: &Env, admin: &Address) -> Address {
    env.register(master_factory_wasm::WASM, (admin.clone(),))
}

/// Uploads a contract WASM and returns its hash.
fn upload_wasm(env: &Env, wasm: &[u8]) -> BytesN<32> {
    env.deployer().upload_contract_wasm(wasm)
}

/// Deploys a TokenFactory through `master_factory` and returns its address.
fn deploy_token_factory(
    env: &Env,
    master_factory: &Address,
    admin: &Address,
    salt: BytesN<32>,
) -> Address {
    let client = master_factory_wasm::Client::new(env, master_factory);
    let wasm_hash = upload_wasm(env, token_factory_wasm::WASM);
    client.deploy_token_factory(admin, &wasm_hash, &salt)
}

/// Returns true when a contract event with the given name was published.
fn verify_event_emitted(env: &Env, event_name: &str) -> bool {
    let expected: Val = Symbol::new(env, event_name).into_val(env);
    env.events()
        .all()
        .iter()
        .any(|(_, topics, _)| topics.get(0) == Some(expected.clone()))
}

#[test]
fn test_full_deployment_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    // Step 1: deploy MasterFactory.
    let master_factory = deploy_master_factory(&env, &admin);
    let master = master_factory_wasm::Client::new(&env, &master_factory);
    assert_eq!(master.get_admin(), admin);

    // Step 2: deploy TokenFactory through MasterFactory.
    let token_factory = deploy_token_factory(
        &env,
        &master_factory,
        &admin,
        BytesN::from_array(&env, &[1u8; 32]),
    );
    assert_eq!(master.get_token_factory(), Some(token_factory.clone()));

    // Step 3: configure TokenFactory with the real Allowlist WASM hash.
    let token_factory_client = token_factory_wasm::Client::new(&env, &token_factory);
    let allowlist_hash = upload_wasm(&env, fungible_allowlist_wasm::WASM);
    token_factory_client.set_allowlist_wasm(&admin, &allowlist_hash);

    // Step 4: deploy one Allowlist token.
    let manager = Address::generate(&env);
    let config = token_factory_wasm::TokenConfig {
        token_type: token_factory_wasm::TokenType::Allowlist,
        admin: admin.clone(),
        manager,
        initial_supply: 1_000_000,
        cap: None,
        name: String::from_str(&env, "Integration Token"),
        symbol: String::from_str(&env, "ITKN"),
        decimals: 7,
        salt: BytesN::from_array(&env, &[2u8; 32]),
        asset: None,
        decimals_offset: None,
    };
    let token_address = token_factory_client.deploy_token(&admin, &config);

    // Step 5: the returned address is recorded and the event fired.
    assert_eq!(token_factory_client.get_token_count(), 1);
    let tokens = token_factory_client.get_deployed_tokens();
    assert_eq!(tokens.get(0).unwrap().address, token_address);
    assert!(verify_event_emitted(&env, "token_deployed_event"));

    // Step 6: interact with the deployed token.
    let token = fungible_allowlist_wasm::Client::new(&env, &token_address);
    assert_eq!(token.total_supply(), 1_000_000);
}

#[test]
fn test_salt_deduplication_rejects_a_reused_salt() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let master_factory = deploy_master_factory(&env, &admin);
    let master = master_factory_wasm::Client::new(&env, &master_factory);

    let salt = BytesN::from_array(&env, &[3u8; 32]);
    let token_factory = deploy_token_factory(&env, &master_factory, &admin, salt.clone());
    assert_eq!(master.get_token_factory(), Some(token_factory));

    // A second deployment with the same salt is rejected before the
    // already-deployed guard can fire.
    let token_factory_hash = upload_wasm(&env, token_factory_wasm::WASM);
    let err: master_factory_wasm::MasterFactoryError = master
        .try_deploy_token_factory(&admin, &token_factory_hash, &salt)
        .unwrap_err()
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(err, master_factory_wasm::MasterFactoryError::DuplicateSalt);
}

#[test]
fn test_pause_blocks_deployment_and_unpause_restores_it() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let master_factory = deploy_master_factory(&env, &admin);
    let master = master_factory_wasm::Client::new(&env, &master_factory);

    let token_factory_hash = upload_wasm(&env, token_factory_wasm::WASM);
    let salt = BytesN::from_array(&env, &[4u8; 32]);

    master.pause(&admin);
    let err: master_factory_wasm::MasterFactoryError = master
        .try_deploy_token_factory(&admin, &token_factory_hash, &salt)
        .unwrap_err()
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(err, master_factory_wasm::MasterFactoryError::ContractPaused);

    master.unpause(&admin);
    let token_factory = master.deploy_token_factory(&admin, &token_factory_hash, &salt);
    assert_eq!(master.get_token_factory(), Some(token_factory));
}

#[test]
fn test_admin_transfer_flow_across_master_factory() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let master_factory = deploy_master_factory(&env, &admin);
    let master = master_factory_wasm::Client::new(&env, &master_factory);

    master.initiate_admin_transfer(&admin, &new_admin);
    assert_eq!(master.get_pending_admin(), Some(new_admin.clone()));

    master.accept_admin_transfer(&new_admin);
    assert_eq!(master.get_admin(), new_admin);
    assert_eq!(master.get_pending_admin(), None);
}
