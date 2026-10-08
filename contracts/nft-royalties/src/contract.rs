//! Non-Fungible Royalties Example Contract.
//!
//! Demonstrates an example usage of the Royalties extension, allowing for
//! setting and querying royalty information for NFTs following the ERC2981
//! standard.

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address,
    Env, String,
};
use stellar_access::access_control::{self as access_control, AccessControl};
use stellar_macros::{default_impl, only_admin, only_role};
use stellar_tokens::non_fungible::{royalties::NonFungibleRoyalties, Base, NonFungibleToken};

#[contract]
pub struct ExampleContract;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ExampleContractError {
    InvalidBasisPoints = 1,
}

/// ERC-2981 basis-point denominator: 10_000 == 100%. Values above it would
/// produce royalty payouts exceeding the sale price.
const MAX_BASIS_POINTS: u32 = 10_000;

fn check_basis_points(e: &Env, basis_points: u32) {
    if basis_points > MAX_BASIS_POINTS {
        panic_with_error!(e, ExampleContractError::InvalidBasisPoints);
    }
}

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(e: &Env, admin: Address, manager: Address, base_uri: String, name: String, symbol: String) {
        Base::set_metadata(e, base_uri, name, symbol);

        // Set default royalty for the entire collection (10%)
        Base::set_default_royalty(e, &admin, 1000);

        access_control::set_admin(e, &admin);

        // create a role "manager" and grant it to `manager`
        access_control::grant_role_no_auth(e, &admin, &manager, &symbol_short!("manager"));
    }

    #[only_admin]
    pub fn mint(e: &Env, to: Address) -> u32 {
        // Mint token with sequential ID
        Base::sequential_mint(e, &to)
    }

    #[only_admin]
    pub fn mint_with_royalty(e: &Env, to: Address, receiver: Address, basis_points: u32) -> u32 {
        check_basis_points(e, basis_points);

        // Mint token with sequential ID
        let token_id = Base::sequential_mint(e, &to);

        // Set token-specific royalty
        Base::set_token_royalty(e, token_id, &receiver, basis_points);

        token_id
    }

    pub fn get_royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        Base::royalty_info(e, token_id, sale_price)
    }
}

#[default_impl]
#[contractimpl]
impl NonFungibleToken for ExampleContract {
    type ContractType = Base;
}

#[contractimpl]
impl NonFungibleRoyalties for ExampleContract {
    #[only_role(operator, "manager")]
    fn set_default_royalty(e: &Env, receiver: Address, basis_points: u32, operator: Address) {
        check_basis_points(e, basis_points);
        Base::set_default_royalty(e, &receiver, basis_points);
    }

    #[only_role(operator, "manager")]
    fn set_token_royalty(
        e: &Env,
        token_id: u32,
        receiver: Address,
        basis_points: u32,
        operator: Address,
    ) {
        check_basis_points(e, basis_points);
        Base::set_token_royalty(e, token_id, &receiver, basis_points);
    }

    #[only_role(operator, "manager")]
    fn remove_token_royalty(e: &Env, token_id: u32, operator: Address) {
        Base::remove_token_royalty(e, token_id);
    }

    fn royalty_info(e: &Env, token_id: u32, sale_price: i128) -> (Address, i128) {
        Base::royalty_info(e, token_id, sale_price)
    }
}

#[default_impl]
#[contractimpl]
impl AccessControl for ExampleContract {}
