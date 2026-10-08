extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(e: &Env, admin: &Address, manager: &Address) -> ExampleContractClient<'a> {
    let address = e.register(
        ExampleContract,
        (
            admin,
            manager,
            String::from_str(e, "https://example.com/nft/"),
            String::from_str(e, "Royal NFT"),
            String::from_str(e, "RNFT"),
        ),
    );
    ExampleContractClient::new(e, &address)
}

#[test]
fn test_default_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    // Mint a token
    let token_id = client.mint(&admin);

    // Check royalty info (should use default 10%)
    let (receiver, amount) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(receiver, admin);
    assert_eq!(amount, 100); // 10% of 1000
}

#[test]
fn test_token_specific_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let royalty_receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    // Mint a token with specific royalty (5%)
    let token_id = client.mint_with_royalty(&admin, &royalty_receiver, &500);

    // Check royalty info
    let (receiver, amount) = client.get_royalty_info(&token_id, &2000);
    assert_eq!(receiver, royalty_receiver);
    assert_eq!(amount, 100); // 5% of 2000

    // Mint a regular token (should use default royalty)
    let regular_token_id = client.mint(&admin);

    // Check royalty info for regular token
    let (receiver, amount) = client.get_royalty_info(&regular_token_id, &2000);
    assert_eq!(receiver, admin);
    assert_eq!(amount, 200); // 10% of 2000
}

#[test]
fn test_zero_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let royalty_receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    // Mint a token with zero royalty
    let token_id = client.mint_with_royalty(&admin, &royalty_receiver, &0);

    // Check royalty info
    let (receiver, amount) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(receiver, royalty_receiver);
    assert_eq!(amount, 0); // 0% royalty
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn mint_with_royalty_rejects_over_10000_bps() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();
    client.mint_with_royalty(&admin, &receiver, &10_001);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn set_default_royalty_rejects_over_10000_bps() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();
    client.set_default_royalty(&receiver, &10_001, &manager);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn set_token_royalty_rejects_over_10000_bps() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();
    let token_id = client.mint(&admin);
    client.set_token_royalty(&token_id, &receiver, &10_001, &manager);
}
