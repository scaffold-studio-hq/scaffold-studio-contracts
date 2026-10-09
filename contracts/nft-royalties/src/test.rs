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
            String::from_str(e, "Royalty NFT"),
            String::from_str(e, "RNFT"),
        ),
    );
    // The deployed royalty contract requires five constructor parameters.
    let address = e.register(ExampleContract, (
        admin, manager,
        String::from_str(e, "https://example.test/royalty/"),
        String::from_str(e, "Royalty NFT"),
        String::from_str(e, "ROY"),
    ));
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

// ===== Role-gated royalty setters (`#[only_role(operator, "manager")]`) =====

/// The `manager` account holds the `manager` role granted in the constructor,
/// so it may update the collection-wide default royalty.
#[test]
fn test_manager_can_set_default_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let new_receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    client.set_default_royalty(&new_receiver, &2500, &manager);

    let token_id = client.mint(&admin);
    let (receiver, amount) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(receiver, new_receiver);
    assert_eq!(amount, 250); // 25% of 1000
}

/// `admin` is the access-control admin but does *not* hold the `manager`
/// role, so the setter is rejected with `AccessControlError::Unauthorized`
/// (Contract, #2000).
#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn test_non_manager_cannot_set_default_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    client.set_default_royalty(&receiver, &2500, &admin);
}

/// A token-specific royalty set by the manager takes precedence, and removing
/// it falls back to the collection default.
#[test]
fn test_manager_can_set_and_remove_token_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    let token_id = client.mint(&admin);
    client.set_token_royalty(&token_id, &receiver, &800, &manager);

    let (r, a) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(r, receiver);
    assert_eq!(a, 80); // 8% of 1000

    // Removing the token royalty falls back to the constructor default
    // (admin, 10%).
    client.remove_token_royalty(&token_id, &manager);

    let (r2, a2) = client.get_royalty_info(&token_id, &1000);
    assert_eq!(r2, admin);
    assert_eq!(a2, 100); // 10% of 1000
}

/// A non-manager cannot set a token-specific royalty either.
#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn test_non_manager_cannot_set_token_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let receiver = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);

    e.mock_all_auths();

    let token_id = client.mint(&admin);
    client.set_token_royalty(&token_id, &receiver, &800, &admin);
#[test]
fn manager_can_update_and_reset_royalties() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let default_to = Address::generate(&e);
    let per_token_to = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();
    let token = client.mint(&admin);
    client.set_default_royalty(&default_to, &750, &manager);
    assert_eq!(client.get_royalty_info(&token, &1000), (default_to.clone(), 75));
    client.set_token_royalty(&token, &per_token_to, &1250, &manager);
    assert_eq!(client.get_royalty_info(&token, &1000), (per_token_to, 125));
    client.remove_token_royalty(&token, &manager);
    assert_eq!(client.get_royalty_info(&token, &1000), (default_to, 75));
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn outsider_cannot_change_default_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let outsider = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();
    client.set_default_royalty(&outsider, &500, &outsider);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn outsider_cannot_set_token_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let outsider = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();
    let token = client.mint(&admin);
    client.set_token_royalty(&token, &outsider, &500, &outsider);
}

#[test]
#[should_panic(expected = "Error(Contract, #2000)")]
fn outsider_cannot_remove_token_royalty() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let outsider = Address::generate(&e);
    let client = create_client(&e, &admin, &manager);
    e.mock_all_auths();
    let token = client.mint(&admin);
    client.set_token_royalty(&token, &manager, &500, &manager);
    client.remove_token_royalty(&token, &outsider);
}
