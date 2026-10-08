extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    initial_supply: &i128,
    cap: &i128,
) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "Capped Token");
    let symbol = String::from_str(e, "CAP");
    let decimals = 7;
    let address = e.register(
        ExampleContract,
        (
            admin,
            manager,
            initial_supply,
            cap,
            name,
            symbol,
            decimals,
        ),
    );
    ExampleContractClient::new(e, &address)
}

#[test]
fn mint_under_cap() {
    let e = Env::default();
    let cap = 1000;
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_supply = 0;
    let client = create_client(&e, &admin, &manager, &initial_supply, &cap);
    let user = Address::generate(&e);

    client.mint(&user, &500);

    assert_eq!(client.balance(&user), 500);
    assert_eq!(client.total_supply(), 500);
}

#[test]
fn mint_exact_cap() {
    let e = Env::default();
    let cap = 1000;
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_supply = 0;
    let client = create_client(&e, &admin, &manager, &initial_supply, &cap);
    let user = Address::generate(&e);

    client.mint(&user, &1000);

    assert_eq!(client.balance(&user), 1000);
    assert_eq!(client.total_supply(), 1000);
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn mint_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_supply = 0;
    let client = create_client(&e, &admin, &manager, &initial_supply, &cap);
    let user = Address::generate(&e);

    // Attempt to mint 1001 tokens (would exceed cap)
    client.mint(&user, &1001); // This should panic
}

#[test]
#[should_panic(expected = "Error(Contract, #106)")]
fn mint_multiple_exceeds_cap() {
    let e = Env::default();
    let cap = 1000;
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let initial_supply = 0;
    let client = create_client(&e, &admin, &manager, &initial_supply, &cap);
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
    let initial_supply = 0;
    let name = String::from_str(&e, "Capped Token");
    let symbol = String::from_str(&e, "CAP");
    let decimals = 7;
    let address = e.register(
        ExampleContract,
        (
            &admin,
            &manager,
            &initial_supply,
            &cap,
            name,
            symbol,
            decimals,
        ),
    );
    let client = token::Client::new(&e, &address);
    let user = Address::generate(&e);

    assert_eq!(client.balance(&user), 0);
}
