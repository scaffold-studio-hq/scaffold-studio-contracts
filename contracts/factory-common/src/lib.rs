#![no_std]

//! Shared factory admin plumbing.
//!
//! `master-factory`, `token-factory`, `nft-factory` and `governance-factory` all
//! re-implement the same emergency-pause and two-step admin-transfer logic. That
//! logic lives here, exactly once: every factory keeps its public surface (and
//! its events) but delegates the storage, authorization and error handling to
//! this module.
//!
//! Each factory calls these helpers from its own `#[contractimpl]` block, for
//! example:
//!
//! ```ignore
//! pub fn pause(e: Env, admin: Address) {
//!     factory_common::pause(&e, &admin);
//!     ContractPausedEvent { admin }.publish(&e);
//! }
//! ```

use soroban_sdk::{contracterror, contracttype, panic_with_error, Address, Env};

/// Storage keys owned by the shared admin/pause surface.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdminKey {
    /// Current admin address.
    Admin,
    /// Address that has been offered the admin role, if a transfer is in flight.
    PendingAdmin,
    /// Emergency stop flag.
    Paused,
}

/// Errors raised by the shared admin/pause surface.
///
/// Defined once, so all four factories report the same codes.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FactoryAdminError {
    NotAdmin = 1,
    AdminNotSet = 2,
    NoPendingAdmin = 3,
    NotPendingAdmin = 4,
    ContractPaused = 5,
}

/// Store the initial admin address.
pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&AdminKey::Admin, admin);
}

/// Get the current admin address.
///
/// # Panics
/// Panics with [`FactoryAdminError::AdminNotSet`] if no admin has been stored.
pub fn get_admin(e: &Env) -> Address {
    e.storage()
        .instance()
        .get(&AdminKey::Admin)
        .unwrap_or_else(|| panic_with_error!(e, FactoryAdminError::AdminNotSet))
}

/// Store the emergency-stop flag.
pub fn set_paused(e: &Env, paused: bool) {
    e.storage().instance().set(&AdminKey::Paused, &paused);
}

/// Whether the contract is currently paused.
pub fn is_paused(e: &Env) -> bool {
    e.storage()
        .instance()
        .get(&AdminKey::Paused)
        .unwrap_or(false)
}

/// Reject a call while the contract is paused.
///
/// # Panics
/// Panics with [`FactoryAdminError::ContractPaused`] when paused.
pub fn require_not_paused(e: &Env) {
    if is_paused(e) {
        panic_with_error!(e, FactoryAdminError::ContractPaused);
    }
}

/// Reject a call from anyone but the stored admin.
///
/// # Panics
/// Panics with [`FactoryAdminError::AdminNotSet`] when no admin is stored, and
/// with [`FactoryAdminError::NotAdmin`] when `address` is not the admin.
pub fn require_admin(e: &Env, address: &Address) {
    let admin: Address = e
        .storage()
        .instance()
        .get(&AdminKey::Admin)
        .unwrap_or_else(|| panic_with_error!(e, FactoryAdminError::AdminNotSet));

    if admin != *address {
        panic_with_error!(e, FactoryAdminError::NotAdmin);
    }
}

/// Pause the contract (emergency stop).
pub fn pause(e: &Env, admin: &Address) {
    admin.require_auth();
    require_admin(e, admin);
    set_paused(e, true);
}

/// Unpause the contract.
pub fn unpause(e: &Env, admin: &Address) {
    admin.require_auth();
    require_admin(e, admin);
    set_paused(e, false);
}

/// Offer the admin role to `new_admin` (step 1 of 2).
pub fn initiate_admin_transfer(e: &Env, current_admin: &Address, new_admin: &Address) {
    current_admin.require_auth();
    require_admin(e, current_admin);

    e.storage().instance().set(&AdminKey::PendingAdmin, new_admin);
}

/// Accept a pending admin transfer (step 2 of 2).
///
/// # Panics
/// Panics with [`FactoryAdminError::NoPendingAdmin`] when there is nothing to
/// accept, and with [`FactoryAdminError::NotPendingAdmin`] when `new_admin` is
/// not the pending admin.
pub fn accept_admin_transfer(e: &Env, new_admin: &Address) {
    new_admin.require_auth();

    let pending_admin: Address = e
        .storage()
        .instance()
        .get(&AdminKey::PendingAdmin)
        .unwrap_or_else(|| panic_with_error!(e, FactoryAdminError::NoPendingAdmin));

    if pending_admin != *new_admin {
        panic_with_error!(e, FactoryAdminError::NotPendingAdmin);
    }

    e.storage().instance().set(&AdminKey::Admin, new_admin);
    e.storage().instance().remove(&AdminKey::PendingAdmin);
}

/// Cancel a pending admin transfer.
pub fn cancel_admin_transfer(e: &Env, current_admin: &Address) {
    current_admin.require_auth();
    require_admin(e, current_admin);

    e.storage().instance().remove(&AdminKey::PendingAdmin);
}

/// The pending admin address, if a transfer is in flight.
pub fn get_pending_admin(e: &Env) -> Option<Address> {
    e.storage().instance().get(&AdminKey::PendingAdmin)
}
