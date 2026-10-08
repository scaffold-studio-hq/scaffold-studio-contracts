//! Capped Example Contract.
//!
//! Demonstrates an example usage of `capped` module by
//! implementing a capped mint mechanism, and setting the maximum supply
//! at the constructor.
//!
//! Minting is restricted to the owner recorded at construction (the `admin`
//! argument): every `mint` call must be authorized by that address.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};
use stellar_tokens::fungible::{
    capped::{check_cap, set_cap},
    Base, FungibleToken,
};

/// Storage key for the address allowed to mint.
pub const OWNER: Symbol = symbol_short!("OWNER");

#[contract]
pub struct ExampleContract;

#[contractimpl]
impl ExampleContract {
    pub fn __constructor(
        e: &Env,
        admin: Address,
        manager: Address,
        initial_supply: i128,
        cap: i128,
        name: String,
        symbol: String,
        decimals: u32,
    ) {
        Base::set_metadata(e, decimals, name, symbol);
        set_cap(e, cap);

        // Mint initial supply to admin
        Base::mint(e, &admin, initial_supply);

        // Record the owner allowed to mint additional supply.
        e.storage().instance().set(&OWNER, &admin);

        // Note: manager parameter included for consistency with other token types
        let _ = manager; // Silence unused warning
    }

    pub fn mint(e: &Env, account: Address, amount: i128) {
        // When `ownable` module is available, the following check should be
        // equivalent to: `ownable::only_owner(&e);`
        let owner: Address = e
            .storage()
            .instance()
            .get(&OWNER)
            .expect("owner should be set");
        owner.require_auth();

        check_cap(e, amount);
        Base::mint(e, &account, amount);
    }
}

#[contractimpl]
impl FungibleToken for ExampleContract {
    type ContractType = Base;

    fn total_supply(e: &Env) -> i128 {
        Self::ContractType::total_supply(e)
    }

    fn balance(e: &Env, account: Address) -> i128 {
        Self::ContractType::balance(e, &account)
    }

    fn allowance(e: &Env, owner: Address, spender: Address) -> i128 {
        Self::ContractType::allowance(e, &owner, &spender)
    }

    fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer(e, &from, &to, amount);
    }

    fn transfer_from(e: &Env, spender: Address, from: Address, to: Address, amount: i128) {
        Self::ContractType::transfer_from(e, &spender, &from, &to, amount);
    }

    fn approve(e: &Env, owner: Address, spender: Address, amount: i128, live_until_ledger: u32) {
        Self::ContractType::approve(e, &owner, &spender, amount, live_until_ledger);
    }

    fn decimals(e: &Env) -> u32 {
        Self::ContractType::decimals(e)
    }

    fn name(e: &Env) -> String {
        Self::ContractType::name(e)
    }

    fn symbol(e: &Env) -> String {
        Self::ContractType::symbol(e)
    }
}
