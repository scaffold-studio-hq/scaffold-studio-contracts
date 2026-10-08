extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, cap: &i128) -> (ExampleContractClient<'a>, Address) {
    let admin = Address::generate(e);
    let manager = Address::generate(e);
    let address = e.register(
        ExampleContract,
        (
            admin.clone(),
            manager,
            0i128,
            *cap,
            String::from_str(e, "Capped Token"),
            String::from_str(e, "CAP"),
            18u32,
        ),
    );
    (ExampleContractClient::new(e, &address), admin)
}

#[test]
fn mint_under_cap() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    let user = Address::generate(&e);

    e.mock_all_auths();
    client.mint(&user, &500);

    assert_eq!(client.balance(&user), 500);
    assert_eq!(client.total_supply(), 500);
}

#[test]
fn mint_exact_cap() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    let user = Address::generate(&e);

    e.mock_all_auths();
    client.mint(&user, &1000);

    assert_eq!(client.balance(&user), 1000);
    assert_eq!(client.total_supply(), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn mint_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    e.mock_all_auths();
    let user = Address::generate(&e);

    // Attempt to mint 1001 tokens (would exceed cap)
    client.mint(&user, &1001); // This should panic
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn mint_multiple_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    e.mock_all_auths();
    let user = Address::generate(&e);

    // Mint 600 tokens first
    client.mint(&user, &600);

    assert_eq!(client.balance(&user), 600);
    assert_eq!(client.total_supply(), 600);

    // Attempt to mint 500 more tokens (would exceed cap)
    client.mint(&user, &500); // This should panic
}

#[test]
fn test_token_interface() {
    let e = Env::default();
    let cap = 1000_i128;

    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let address = e.register(
        ExampleContract,
        (
            admin,
            manager,
            0i128,
            cap,
            String::from_str(&e, "Capped Token"),
            String::from_str(&e, "CAP"),
            18u32,
        ),
    );
    let client = token::Client::new(&e, &address);
    let user = Address::generate(&e);

    assert_eq!(client.balance(&user), 0);
}

#[test]
fn mint_without_owner_auth_fails() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    let user = Address::generate(&e);

    // No mock auths registered -> owner.require_auth() must fail.
    assert!(client.try_mint(&user, &10).is_err());
}

#[test]
fn mint_with_owner_auth_succeeds() {
    let e = Env::default();
    let cap = 1000;
    let (client, _admin) = create_client(&e, &cap);
    let user = Address::generate(&e);

    e.mock_all_auths();
    client.mint(&user, &10);
    assert_eq!(client.balance(&user), 10);
}
