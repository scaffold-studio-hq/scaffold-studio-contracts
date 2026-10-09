extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, owner: &Address, initial_supply: i128) -> ExampleContractClient<'a> {
    let manager = Address::generate(e);
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(
        ExampleContract,
        (owner, manager, initial_supply, name, symbol, 18u32),
    let address = e.register(
        ExampleContract,
        (
            owner.clone(),
            owner.clone(),
            initial_supply,
            String::from_str(e, "Pausable Token"),
            String::from_str(e, "PAUSE"),
            7u32,
            manager,
            initial_supply,
            String::from_str(e, "My Token"),
            String::from_str(e, "TKN"),
            18u32,
        ),
    );
    ExampleContractClient::new(e, &address)
}

#[test]
fn initial_state() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    assert_eq!(client.total_supply(), 1000);
    assert_eq!(client.balance(&owner), 1000);
    assert_eq!(client.symbol(), String::from_str(&e, "PAUSE"));
    assert_eq!(client.name(), String::from_str(&e, "Pausable Token"));
    assert_eq!(client.decimals(), 7);
    assert!(!client.paused());
}

#[test]
fn transfer_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.transfer(&owner, &recipient, &100);
    assert_eq!(client.balance(&owner), 900);
    assert_eq!(client.balance(&recipient), 100);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn transfer_fails_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.pause(&owner);
    client.transfer(&owner, &recipient, &100);
}

#[test]
fn transfer_from_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.approve(&owner, &spender, &200, &100);
    client.transfer_from(&spender, &owner, &recipient, &200);
    assert_eq!(client.balance(&owner), 800);
    assert_eq!(client.balance(&recipient), 200);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn transfer_from_fails_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.pause(&owner);
    client.transfer_from(&spender, &owner, &recipient, &200);
}

#[test]
fn mint_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.mint(&owner, &500);
    assert_eq!(client.total_supply(), 1500);
    assert_eq!(client.balance(&owner), 1500);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn mint_fails_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.pause(&owner);
    client.mint(&owner, &500);
}

#[test]
fn burn_works() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.burn(&owner, &200);
    assert_eq!(client.total_supply(), 800);
    assert_eq!(client.balance(&owner), 800);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn burn_fails_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.pause(&owner);
    client.burn(&owner, &200);
}

#[test]
#[should_panic(expected = "Error(Contract, #1000)")]
fn approve_fails_when_paused() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.pause(&owner);
    client.approve(&owner, &spender, &100, &100);
}

#[test]
fn approved_allowance_still_works_after_unpause() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let spender = Address::generate(&e);
    let recipient = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    client.approve(&owner, &spender, &150, &100);
    client.pause(&owner);
    assert_eq!(client.allowance(&owner, &spender), 150);
    client.unpause(&owner);
    client.transfer_from(&spender, &owner, &recipient, &100);
    assert_eq!(client.balance(&recipient), 100);
    assert_eq!(client.allowance(&owner, &spender), 50);
#[should_panic(expected = "Error(Contract, #2)")]
fn mint_fails_with_defined_error_when_owner_missing() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    e.as_contract(&client.address, || {
        e.storage().instance().remove(&crate::contract::OWNER);
    });

    client.mint(&owner, &500);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn pause_fails_with_defined_error_when_owner_missing() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    e.as_contract(&client.address, || {
        e.storage().instance().remove(&crate::contract::OWNER);
    });

    client.pause(&owner);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn unpause_fails_with_defined_error_when_owner_missing() {
    let e = Env::default();
    let owner = Address::generate(&e);
    let client = create_client(&e, &owner, 1000);

    e.mock_all_auths();
    e.as_contract(&client.address, || {
        e.storage().instance().remove(&crate::contract::OWNER);
    });

    client.unpause(&owner);
}
