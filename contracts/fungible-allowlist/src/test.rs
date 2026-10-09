extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::contract::{ExampleContract, ExampleContractClient};

fn create_client<'a>(
    e: &Env,
    admin: &Address,
    manager: &Address,
    initial_supply: &i128,
) -> ExampleContractClient<'a> {
    let name = String::from_str(e, "Allowlist Token");
    let symbol = String::from_str(e, "ALLOW");
    let decimals = 7;
    let address = e.register(
        ExampleContract,
        (admin, manager, initial_supply, name, symbol, decimals),
    let address = e.register(
        ExampleContract,
        (
            admin,
            manager,
            initial_supply,
            String::from_str(e, "Allowlist Token"),
            String::from_str(e, "ALWL"),
            7u32,
        ),
    );
    let name = String::from_str(e, "My Token");
    let symbol = String::from_str(e, "TKN");
    let address = e.register(ExampleContract, (admin, manager, initial_supply, name, symbol, 7u32));
    let name = String::from_str(e, "AllowList Token");
    let symbol = String::from_str(e, "ALT");
    let decimals = 7;
    let address = e.register(
        ExampleContract,
        (admin, manager, initial_supply, &name, &symbol, decimals),
    );
    ExampleContractClient::new(e, &address)
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_transfer_before_allow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Admin can't transfer to user1 initially (user1 not allowed)
    e.mock_all_auths();
    client.transfer(&admin, &user1, &transfer_amount);
}

#[test]
fn transfer_to_allowed_account_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Now admin can transfer to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_transfer_after_disallow() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Now admin can transfer to user1
    client.transfer(&admin, &user1, &transfer_amount);
    assert_eq!(client.balance(&user1), transfer_amount);

    // Disallow user1
    client.disallow_user(&user1, &manager);
    assert!(!client.allowed(&user1));

    // Admin can't transfer to user1 after disallowing
    client.transfer(&admin, &user1, &100);
}

#[test]
fn allowlist_transfer_from_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user2
    client.allow_user(&user2, &manager);
    assert!(client.allowed(&user2));

    // Now admin can transfer to user1
    client.approve(&admin, &user1, &transfer_amount, &1000);
    client.transfer_from(&user1, &admin, &user2, &transfer_amount);
    assert_eq!(client.balance(&user2), transfer_amount);
}

#[test]
fn allowlist_approve_override_works() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Verify initial state - admin is allowed, others are not
    assert!(client.allowed(&admin));
    assert!(!client.allowed(&user1));
    assert!(!client.allowed(&user2));

    // Allow user1
    client.allow_user(&user1, &manager);
    assert!(client.allowed(&user1));

    // Approve user2 to transfer from user1
    client.approve(&user1, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&user1, &user2), transfer_amount);
}

#[test]
#[should_panic(expected = "Error(Contract, #113)")]
fn cannot_transfer_from_after_disallowing_allowance_owner() {
fn disallowed_spender_cannot_use_stale_allowance() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Allow the spender/recipient so the only allowlist rejection left is the
    // disallowed `from` account.
    client.allow_user(&user1, &manager);
    client.allow_user(&user2, &manager);

    // Grant an allowance before any disallow happens.
    client.approve(&admin, &user1, &transfer_amount, &1000);
    assert_eq!(client.allowance(&admin, &user1), transfer_amount);

    // Disallow the account that owns the tokens the allowance authorizes
    // moving. OZ's `AllowList::transfer_from` re-checks `from` and `to` on
    // every call, so a pre-existing allowance must not survive the disallow.
    client.disallow_user(&admin, &manager);
    assert!(!client.allowed(&admin));

    client.transfer_from(&user1, &admin, &user2, &transfer_amount);
}

#[test]
fn disallowed_spender_can_still_use_allowance() {
    // OpenZeppelin v0.5.1 explicitly exempts the spender from the allowlist
    // ("Note that, spender does not have to be allowed."), so disallowing the
    // spender is *not* a rejection path. This test pins that documented
    // behaviour: the edge case is observed, but with the spender exemption in
    // place rather than an assumed `UserNotAllowed` failure.
    let e = Env::default();
    let admin = Address::generate(&e);
    let manager = Address::generate(&e);
    let spender = Address::generate(&e);
    let user2 = Address::generate(&e);
    let initial_supply = 1_000_000;
    let client = create_client(&e, &admin, &manager, &initial_supply);
    let transfer_amount = 1000;

    e.mock_all_auths();

    // Allow the recipient only.
    client.allow_user(&user2, &manager);
    assert!(!client.allowed(&spender));

    // The owner grants an allowance to a spender that is not allowed.
    client.approve(&admin, &spender, &transfer_amount, &1000);
    assert_eq!(client.allowance(&admin, &spender), transfer_amount);

    // Disallowing the spender has no effect on the allowance.
    client.disallow_user(&spender, &manager);
    assert!(!client.allowed(&spender));

    client.transfer_from(&spender, &admin, &user2, &transfer_amount);
    assert_eq!(client.balance(&user2), transfer_amount);
    // user1 is an allowed recipient and user2 starts out allowed
    client.allow_user(&user1, &manager);
    client.allow_user(&user2, &manager);
    assert!(client.allowed(&user1));
    assert!(client.allowed(&user2));

    // Admin grants user2 an allowance while user2 is allowed
    client.approve(&admin, &user2, &transfer_amount, &1000);
    assert_eq!(client.allowance(&admin, &user2), transfer_amount);

    // Manager disallows the spender after the allowance exists
    client.disallow_user(&user2, &manager);
    assert!(!client.allowed(&user2));

    // The stale allowance must not let user2 move admin's tokens
    client.transfer_from(&user2, &admin, &user1, &transfer_amount);
}
