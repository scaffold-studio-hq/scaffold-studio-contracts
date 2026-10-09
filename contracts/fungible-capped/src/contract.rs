//! Capped Example Contract.
//!
//! Demonstrates an example usage of `capped` module by
//! implementing a capped mint mechanism, and setting the maximum supply
//! at the constructor.
//!
//! **IMPORTANT**: this example is for demonstration purposes, and authorization
//! is not taken into consideration

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String, Symbol};
use stellar_tokens::fungible::{
    capped::{check_cap, set_cap},
    Base, FungibleToken,
};

/// Instance-storage key under which the contract owner (the constructor
/// `admin`) is persisted.
pub const OWNER: Symbol = symbol_short!("OWNER");

/// Instance-storage key under which the contract manager is persisted.
pub const MANAGER: Symbol = symbol_short!("MANAGER");

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

        // Persist the admin as the contract owner so that future privileged
        // operations (e.g. `mint`) have a principal to authorize against.
        e.storage().instance().set(&OWNER, &admin);
        // Persist the manager as a separate role instead of discarding it.
        e.storage().instance().set(&MANAGER, &manager);

        // Mint initial supply to admin
        Base::mint(e, &admin, initial_supply);
    }

    /// Returns the stored contract owner (the constructor `admin`).
    pub fn get_owner(e: &Env) -> Address {
        e.storage().instance().get(&OWNER).expect("owner should be set")
    }

    /// Returns the stored contract manager (the constructor `manager`).
    pub fn get_manager(e: &Env) -> Address {
        e.storage().instance().get(&MANAGER).expect("manager should be set")
    }

    pub fn mint(e: &Env, account: Address, amount: i128) {
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
