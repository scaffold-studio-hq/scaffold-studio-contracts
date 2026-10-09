#![no_std]

use soroban_sdk::{contract, contractevent, contractimpl, contracterror, contracttype, panic_with_error, Address, BytesN, Env, Vec};

/// MasterFactory - Central factory that deploys and manages other factories
///
/// This contract is the entry point for Stellar Studio's factory system.
/// It deploys and tracks TokenFactory, NFTFactory, and GovernanceFactory contracts.

#[contract]
pub struct MasterFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    TokenFactory,
    NFTFactory,
    GovernanceFactory,
    DeployedFactory(u32),        // Indexed deployed-factory record
    FactoryCount,
    Deploying,
    UsedSalts(BytesN<32>),
    DeploymentsInBlock(u32),
    Paused,
    // Append only: keep existing Soroban storage-key discriminants stable.
    FactoryCount,
    DeployedFactory(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactoryInfo {
    pub address: Address,
    pub factory_type: FactoryType,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactoryType {
    Token,
    NFT,
    Governance,
}

#[contractevent]
pub struct FactoryDeployedEvent {
    pub factory_address: Address,
    pub factory_type: FactoryType,
    pub deployer: Address,
    pub timestamp: u64,
}

#[contractevent]
pub struct ContractPausedEvent {
    pub admin: Address,
}

#[contractevent]
pub struct ContractUnpausedEvent {
    pub admin: Address,
}

#[contractevent]
pub struct ContractUpgradedEvent {
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent]
pub struct AdminTransferInitiatedEvent {
    pub new_admin: Address,
}

#[contractevent]
pub struct AdminTransferredEvent {
    pub new_admin: Address,
}

#[contractevent]
pub struct AdminTransferCancelledEvent {
    pub admin: Address,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MasterFactoryError {
    FactoryAlreadyDeployed = 2,
    AdminNotSet = 4,
    FactoryNotFound = 3,
    Reentrancy = 5,
    DuplicateSalt = 6,
    RateLimitExceeded = 7,
    CounterOverflow = 11,
}

// Approximate ledger counts at 5s/ledger. Clamp at runtime to network max TTL.
const USED_SALT_REFRESH_THRESHOLD: u32 = 30 * 17_280;
const USED_SALT_TTL_TARGET: u32 = 90 * 17_280;

#[contractimpl]
impl MasterFactory {
    /// Number of ledgers below which a `UsedSalts` entry's TTL is refreshed.
    /// Approximately 30 days at ~5s per ledger.
    const SALT_TTL_THRESHOLD: u32 = 518_400;
    /// TTL (in ledgers) a `UsedSalts` entry is extended to. Approximately one
    /// year at ~5s per ledger, so a recorded salt cannot be forgotten by
    /// archival while the factory is expected to live.
    const SALT_TTL_EXTEND_TO: u32 = 6_307_200;
    /// Number of ledgers below which the per-block deployment counter TTL is
    /// refreshed. The counter is only meaningful for its own ledger.
    const DEPLOYMENT_TTL_THRESHOLD: u32 = 100;
    /// TTL (in ledgers) the per-block deployment counter is extended to.
    const DEPLOYMENT_TTL_EXTEND_TO: u32 = 1_000;
    /// Number of ledgers below which a deployed-factory record's TTL is
    /// refreshed. Approximately 30 days at ~5s per ledger.
    const RECORD_TTL_THRESHOLD: u32 = 518_400;
    /// TTL (in ledgers) a deployed-factory record is extended to. Approximately
    /// one year at ~5s per ledger.
    const RECORD_TTL_EXTEND_TO: u32 = 6_307_200;

    /// Initialize MasterFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        factory_common::set_admin(&e, &admin);

        // Deployed factories are stored under indexed persistent keys; only the
        // small counter lives in instance storage.
        // New deployments use individually addressable persistent entries.
        // Legacy DeployedFactories remains available as read-only fallback.
        e.storage().instance().set(&DataKey::FactoryCount, &0u32);
        e.storage().instance().set(&DataKey::Deploying, &false);
        factory_common::set_paused(&e, false);
    }

    /// Deploy TokenFactory contract
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function (must be admin)
    /// * `wasm_hash` - WASM hash of the TokenFactory contract
    /// * `salt` - Salt for deterministic address generation
    ///
    /// # Returns
    /// Address of the deployed TokenFactory
    pub fn deploy_token_factory(
        e: Env,
        deployer: Address,
        wasm_hash: BytesN<32>,
        salt: BytesN<32>,
    ) -> Address {
        // Require authorization
        deployer.require_auth();

        // Check admin
        factory_common::require_admin(&e, &deployer);

        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Reentrancy guard
        let is_deploying = e.storage().instance().get(&DataKey::Deploying).unwrap_or(false);
        if is_deploying {
            panic_with_error!(&e, MasterFactoryError::Reentrancy);
        }
        e.storage().instance().set(&DataKey::Deploying, &true);

        // Rate limiting - max 10 deployments per block
        let current_block = e.ledger().sequence();
        let deployments_key = DataKey::DeploymentsInBlock(current_block);
        let deployments_count = e.storage().temporary().get(&deployments_key).unwrap_or(0u32);

        if deployments_count >= 10 {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::RateLimitExceeded);
        }

        // Check for salt reuse
        let salt_key = DataKey::UsedSalts(salt.clone());
        if Self::salt_is_used(&e, &salt_key) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::DuplicateSalt);
        }

        // Check if already deployed
        if e.storage().instance().has(&DataKey::TokenFactory) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::FactoryAlreadyDeployed);
        }

        // Deploy using deployer pattern, passing deployer as admin
        let factory_address = e.deployer()
            .with_address(e.current_contract_address(), salt.clone())
            .deploy_v2(wasm_hash, (deployer.clone(),));

        // Mark salt as used
        Self::record_salt_used(&e, &salt_key);
        e.storage().persistent().set(&salt_key, &true);
        Self::extend_used_salt_ttl(&e, &salt_key);

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
        // Keyed by ledger sequence: the temporary counter is intentionally short-lived.
        e.storage().temporary().set(&deployments_key, &new_deployments_count);
        e.storage().temporary().extend_ttl(
            &deployments_key,
            Self::DEPLOYMENT_TTL_THRESHOLD,
            Self::DEPLOYMENT_TTL_EXTEND_TO,
        );

        // Store factory address
        e.storage().instance().set(&DataKey::TokenFactory, &factory_address);

        // Add to deployed factories list
        let factory_info = FactoryInfo {
            address: factory_address.clone(),
            factory_type: FactoryType::Token,
            timestamp: e.ledger().timestamp(),
        };

        Self::append_factory(&e, factory_info);
        // Stable append index, no whole-vector rewrite on deployment.
        let count = Self::get_factory_count(e.clone());
        let next_count = count.checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::CounterOverflow));
        e.storage().persistent().set(&DataKey::DeployedFactory(count), &factory_info);
        e.storage().instance().set(&DataKey::FactoryCount, &next_count);

        // Emit event
        FactoryDeployedEvent {
            factory_address: factory_address.clone(),
            factory_type: FactoryType::Token,
            deployer: deployer.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        // Clear reentrancy guard
        e.storage().instance().set(&DataKey::Deploying, &false);

        factory_address
    }

    /// Deploy NFTFactory contract
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function (must be admin)
    /// * `wasm_hash` - WASM hash of the NFTFactory contract
    /// * `salt` - Salt for deterministic address generation
    ///
    /// # Returns
    /// Address of the deployed NFTFactory
    pub fn deploy_nft_factory(
        e: Env,
        deployer: Address,
        wasm_hash: BytesN<32>,
        salt: BytesN<32>,
    ) -> Address {
        deployer.require_auth();
        factory_common::require_admin(&e, &deployer);

        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Reentrancy guard
        let is_deploying = e.storage().instance().get(&DataKey::Deploying).unwrap_or(false);
        if is_deploying {
            panic_with_error!(&e, MasterFactoryError::Reentrancy);
        }
        e.storage().instance().set(&DataKey::Deploying, &true);

        // Rate limiting
        let current_block = e.ledger().sequence();
        let deployments_key = DataKey::DeploymentsInBlock(current_block);
        let deployments_count = e.storage().temporary().get(&deployments_key).unwrap_or(0u32);

        if deployments_count >= 10 {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::RateLimitExceeded);
        }

        // Check for salt reuse
        let salt_key = DataKey::UsedSalts(salt.clone());
        if Self::salt_is_used(&e, &salt_key) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::DuplicateSalt);
        }

        if e.storage().instance().has(&DataKey::NFTFactory) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::FactoryAlreadyDeployed);
        }

        // Deploy using deployer pattern, passing deployer as admin
        let factory_address = e.deployer()
            .with_address(e.current_contract_address(), salt.clone())
            .deploy_v2(wasm_hash, (deployer.clone(),));

        // Mark salt as used
        Self::record_salt_used(&e, &salt_key);
        e.storage().persistent().set(&salt_key, &true);
        Self::extend_used_salt_ttl(&e, &salt_key);

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
        // Keyed by ledger sequence: the temporary counter is intentionally short-lived.
        e.storage().temporary().set(&deployments_key, &new_deployments_count);
        e.storage().temporary().extend_ttl(
            &deployments_key,
            Self::DEPLOYMENT_TTL_THRESHOLD,
            Self::DEPLOYMENT_TTL_EXTEND_TO,
        );

        e.storage().instance().set(&DataKey::NFTFactory, &factory_address);

        let factory_info = FactoryInfo {
            address: factory_address.clone(),
            factory_type: FactoryType::NFT,
            timestamp: e.ledger().timestamp(),
        };

        Self::append_factory(&e, factory_info);
        // Stable append index, no whole-vector rewrite on deployment.
        let count = Self::get_factory_count(e.clone());
        let next_count = count.checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::CounterOverflow));
        e.storage().persistent().set(&DataKey::DeployedFactory(count), &factory_info);
        e.storage().instance().set(&DataKey::FactoryCount, &next_count);

        // Emit event
        FactoryDeployedEvent {
            factory_address: factory_address.clone(),
            factory_type: FactoryType::NFT,
            deployer: deployer.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        // Clear reentrancy guard
        e.storage().instance().set(&DataKey::Deploying, &false);

        factory_address
    }

    /// Deploy GovernanceFactory contract
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function (must be admin)
    /// * `wasm_hash` - WASM hash of the GovernanceFactory contract
    /// * `salt` - Salt for deterministic address generation
    ///
    /// # Returns
    /// Address of the deployed GovernanceFactory
    pub fn deploy_governance_factory(
        e: Env,
        deployer: Address,
        wasm_hash: BytesN<32>,
        salt: BytesN<32>,
    ) -> Address {
        deployer.require_auth();
        factory_common::require_admin(&e, &deployer);

        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Reentrancy guard
        let is_deploying = e.storage().instance().get(&DataKey::Deploying).unwrap_or(false);
        if is_deploying {
            panic_with_error!(&e, MasterFactoryError::Reentrancy);
        }
        e.storage().instance().set(&DataKey::Deploying, &true);

        // Rate limiting
        let current_block = e.ledger().sequence();
        let deployments_key = DataKey::DeploymentsInBlock(current_block);
        let deployments_count = e.storage().temporary().get(&deployments_key).unwrap_or(0u32);

        if deployments_count >= 10 {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::RateLimitExceeded);
        }

        // Check for salt reuse
        let salt_key = DataKey::UsedSalts(salt.clone());
        if Self::salt_is_used(&e, &salt_key) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::DuplicateSalt);
        }

        if e.storage().instance().has(&DataKey::GovernanceFactory) {
            e.storage().instance().set(&DataKey::Deploying, &false);
            panic_with_error!(&e, MasterFactoryError::FactoryAlreadyDeployed);
        }

        // Deploy using deployer pattern, passing deployer as admin
        let factory_address = e.deployer()
            .with_address(e.current_contract_address(), salt.clone())
            .deploy_v2(wasm_hash, (deployer.clone(),));

        // Mark salt as used
        Self::record_salt_used(&e, &salt_key);
        e.storage().persistent().set(&salt_key, &true);
        Self::extend_used_salt_ttl(&e, &salt_key);

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
        // Keyed by ledger sequence: the temporary counter is intentionally short-lived.
        e.storage().temporary().set(&deployments_key, &new_deployments_count);
        e.storage().temporary().extend_ttl(
            &deployments_key,
            Self::DEPLOYMENT_TTL_THRESHOLD,
            Self::DEPLOYMENT_TTL_EXTEND_TO,
        );

        e.storage().instance().set(&DataKey::GovernanceFactory, &factory_address);

        let factory_info = FactoryInfo {
            address: factory_address.clone(),
            factory_type: FactoryType::Governance,
            timestamp: e.ledger().timestamp(),
        };

        Self::append_factory(&e, factory_info);
        // Stable append index, no whole-vector rewrite on deployment.
        let count = Self::get_factory_count(e.clone());
        let next_count = count.checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::CounterOverflow));
        e.storage().persistent().set(&DataKey::DeployedFactory(count), &factory_info);
        e.storage().instance().set(&DataKey::FactoryCount, &next_count);

        // Emit event
        FactoryDeployedEvent {
            factory_address: factory_address.clone(),
            factory_type: FactoryType::Governance,
            deployer: deployer.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        // Clear reentrancy guard
        e.storage().instance().set(&DataKey::Deploying, &false);

        factory_address
    }

    /// Report whether a deterministic deployment salt has been reserved.
    /// A successful state-changing call also renews its persistence TTL.
    /// Simulated read-only RPC calls do not commit TTL changes.
    pub fn is_salt_used(e: Env, salt: BytesN<32>) -> bool {
        Self::salt_is_used(&e, &DataKey::UsedSalts(salt))
    }

    /// Get TokenFactory address
    ///
    /// # Returns
    /// Address of the TokenFactory if deployed, None otherwise
    pub fn get_token_factory(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::TokenFactory)
    }

    /// Get NFTFactory address
    ///
    /// # Returns
    /// Address of the NFTFactory if deployed, None otherwise
    pub fn get_nft_factory(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::NFTFactory)
    }

    /// Get GovernanceFactory address
    ///
    /// # Returns
    /// Address of the GovernanceFactory if deployed, None otherwise
    pub fn get_governance_factory(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::GovernanceFactory)
    }

    /// Get all deployed factories
    ///
    /// # Returns
    /// Vector of FactoryInfo containing all deployed factories
    pub fn get_deployed_factories(e: Env) -> Vec<FactoryInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::FactoryCount).unwrap_or(0);
        let mut factories = Vec::new(&e);
        for i in 0..count {
            if let Some(factory) = e
                .storage()
                .persistent()
                .get::<_, FactoryInfo>(&DataKey::DeployedFactory(i))
            {
                factories.push_back(factory);
            }
        }
        factories
    }

    /// Get a page of deployed factories
    ///
    /// # Arguments
    /// * `start` - Index of the first record to return
    /// * `limit` - Maximum number of records to return
    ///
    /// # Returns
    /// Vector of FactoryInfo for the requested page
    pub fn get_deployed_factories_paginated(e: Env, start: u32, limit: u32) -> Vec<FactoryInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::FactoryCount).unwrap_or(0);
        let mut factories = Vec::new(&e);
        let mut i = start;
        while i < count && factories.len() < limit {
            if let Some(factory) = e
                .storage()
                .persistent()
                .get::<_, FactoryInfo>(&DataKey::DeployedFactory(i))
            {
                factories.push_back(factory);
            }
            i += 1;
        }
        factories
    }

    /// Get total number of deployed factories
    ///
    /// # Returns
    /// Total count of deployed factories
    pub fn get_factory_count(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::FactoryCount).unwrap_or(0)
    }

    /// Upgrade the factory contract to a new WASM hash
    ///
    /// # Arguments
    /// * `new_wasm_hash` - New WASM hash to upgrade to
    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        // Get admin and require their authorization
        let admin = factory_common::get_admin(&e);
        admin.require_auth();

        // Pause contract during upgrade
        factory_common::set_paused(&e, true);

        // Emit upgrade event
        ContractUpgradedEvent {
            new_wasm_hash: new_wasm_hash.clone(),
        }
        .publish(&e);

        e.deployer().update_current_contract_wasm(new_wasm_hash);
        let count = Self::get_factory_count(e.clone());
        Self::deployed_factories_range(&e, 0, count)
    }

    /// Count of indexed factories (or pre-upgrade historical entries).
    pub fn get_factory_count(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::FactoryCount)
            .unwrap_or_else(|| {
                let legacy: Vec<FactoryInfo> = e.storage().instance()
                    .get(&DataKey::DeployedFactories).unwrap_or_else(|| Vec::new(&e));
                legacy.len()
            })
    }

    /// Bounded historical and indexed factory records (up to 100 per page).
    pub fn get_deployed_factories_page(e: Env, start: u32, limit: u32) -> Vec<FactoryInfo> {
        let count = Self::get_factory_count(e.clone());
        let end = start.saturating_add(limit.min(100)).min(count);
        Self::deployed_factories_range(&e, start, end)
    }

    fn deployed_factories_range(e: &Env, start: u32, end: u32) -> Vec<FactoryInfo> {
        let mut result = Vec::new(e);
        let legacy: Vec<FactoryInfo> = e.storage().instance()
            .get(&DataKey::DeployedFactories).unwrap_or_else(|| Vec::new(e));
        for index in start..end {
            let indexed: Option<FactoryInfo> = e.storage().persistent()
                .get(&DataKey::DeployedFactory(index));
            if let Some(info) = indexed.or_else(|| legacy.get(index)) {
                result.push_back(info);
            }
        }
        result
    }

    /// Get admin address
    ///
    /// # Returns
    /// Address of the admin
    pub fn get_admin(e: Env) -> Address {
        factory_common::get_admin(&e)
    }

    /// Get pending admin address
    ///
    /// # Returns
    /// Option containing pending admin address
    pub fn get_pending_admin(e: Env) -> Option<Address> {
        factory_common::get_pending_admin(&e)
    }

    /// Pause contract (emergency stop)
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    pub fn pause(e: Env, admin: Address) {
        factory_common::pause(&e, &admin);

        ContractPausedEvent { admin }.publish(&e);
    }

    /// Unpause contract
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    pub fn unpause(e: Env, admin: Address) {
        factory_common::unpause(&e, &admin);

        ContractUnpausedEvent { admin }.publish(&e);
    }

    /// Initiate admin transfer (step 1 of 2)
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address (must match stored admin)
    /// * `new_admin` - New admin address
    pub fn initiate_admin_transfer(e: Env, current_admin: Address, new_admin: Address) {
        factory_common::initiate_admin_transfer(&e, &current_admin, &new_admin);

        AdminTransferInitiatedEvent { new_admin }.publish(&e);
    }

    /// Accept admin transfer (step 2 of 2)
    ///
    /// # Arguments
    /// * `new_admin` - New admin address accepting the role
    pub fn accept_admin_transfer(e: Env, new_admin: Address) {
        factory_common::accept_admin_transfer(&e, &new_admin);

        AdminTransferredEvent { new_admin }.publish(&e);
    }

    /// Cancel pending admin transfer
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address
    pub fn cancel_admin_transfer(e: Env, current_admin: Address) {
        factory_common::cancel_admin_transfer(&e, &current_admin);

        AdminTransferCancelledEvent {
            admin: current_admin,
        }
        .publish(&e);
    }

    // Record a consumed salt and refresh its persistent TTL so the duplicate
    // guard is not lost to archival.
    fn record_salt_used(e: &Env, salt_key: &DataKey) {
        e.storage().persistent().set(salt_key, &true);
        e.storage().persistent().extend_ttl(
            salt_key,
            Self::SALT_TTL_THRESHOLD,
            Self::SALT_TTL_EXTEND_TO,
        );
    // Append a deployed factory under its own indexed persistent key, bumping
    // the instance-level counter.
    fn append_factory(e: &Env, info: FactoryInfo) {
        let count: u32 = e.storage().instance().get(&DataKey::FactoryCount).unwrap_or(0);
        let new_count = count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(e, MasterFactoryError::CounterOverflow));

        let index_key = DataKey::DeployedFactory(count);
        e.storage().persistent().set(&index_key, &info);
        e.storage().persistent().extend_ttl(
            &index_key,
            Self::RECORD_TTL_THRESHOLD,
            Self::RECORD_TTL_EXTEND_TO,
        );

        e.storage().instance().set(&DataKey::FactoryCount, &new_count);
    fn extend_used_salt_ttl(e: &Env, key: &DataKey) {
        // Network limits can be lower than our preferred 90-day target.
        let target = core::cmp::min(USED_SALT_TTL_TARGET, e.storage().max_ttl());
        let threshold = core::cmp::min(USED_SALT_REFRESH_THRESHOLD, target);
        e.storage().persistent().extend_ttl(key, threshold, target);
    }

    fn salt_is_used(e: &Env, key: &DataKey) -> bool {
        if e.storage().persistent().has(key) {
            Self::extend_used_salt_ttl(e, key);
            true
        } else {
            false
        }
    }

    // Helper function to check admin authorization
    fn require_admin(e: &Env, address: &Address) {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(e, MasterFactoryError::AdminNotSet));

        if admin != *address {
            panic_with_error!(e, MasterFactoryError::NotAdmin);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::storage::Persistent as _, testutils::Address as _, Env,
    };
    use soroban_sdk::{testutils::{Address as _, Ledger as _, storage::Persistent as _}, Env};

    fn setup_master_factory(env: &Env) -> (MasterFactoryClient, Address) {
        let admin = Address::generate(env);
        let contract_id = env.register(MasterFactory, (&admin,));
        let client = MasterFactoryClient::new(env, &contract_id);
        (client, admin)
    }

    #[test]
    fn test_salt_ttl_extended_after_write() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let contract_id = env.register(MasterFactory, (&admin,));
        let salt_key = DataKey::UsedSalts(BytesN::from_array(&env, &[7u8; 32]));

        // Simulate recording a consumed salt the same way the deploy paths do.
        env.as_contract(&contract_id, || {
            MasterFactory::record_salt_used(&env, &salt_key);
        });

        let ttl = env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&salt_key)
        });
        assert!(ttl >= MasterFactory::SALT_TTL_EXTEND_TO - 1);
    fn test_used_salt_ttl_renews_on_repeat_successful_lookup() {
        let env = Env::default();
        // Give new entries a short initial TTL so renewal is observable.
        env.ledger().set_min_persistent_entry_ttl(20);
        let (client, _admin) = setup_master_factory(&env);
        let salt = BytesN::from_array(&env, &[9u8; 32]);
        let key = DataKey::UsedSalts(salt.clone());
        let initial = env.as_contract(&client.address, || {
            env.storage().persistent().set(&key, &true);
            env.storage().persistent().get_ttl(&key)
        });

        assert!(client.is_salt_used(&salt));
        let extended = env.as_contract(&client.address, || {
            env.storage().persistent().get_ttl(&key)
        });
        assert!(extended > initial, "repeated salt check should renew TTL");
        assert!(client.is_salt_used(&salt));
        let repeat = env.as_contract(&client.address, || {
            env.storage().persistent().get_ttl(&key)
        });
        assert!(repeat >= extended);
    }

    // ===== Constructor Tests =====

    #[test]
    fn test_constructor() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(MasterFactory, (&admin,));
        let client = MasterFactoryClient::new(&env, &contract_id);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);

        let factories = client.get_deployed_factories();
        assert_eq!(factories.len(), 0);
    }

    // ===== Query Tests =====

    #[test]
    fn test_get_factories_empty() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(MasterFactory, (&admin,));
        let client = MasterFactoryClient::new(&env, &contract_id);

        assert_eq!(client.get_token_factory(), None);
        assert_eq!(client.get_nft_factory(), None);
        assert_eq!(client.get_governance_factory(), None);
    }

    #[test]
    fn test_get_deployed_factories_empty() {
        let env = Env::default();
        let (client, _admin) = setup_master_factory(&env);

        let factories = client.get_deployed_factories();
        assert_eq!(factories.len(), 0);
    }

    // ===== Authorization Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_deploy_token_factory_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let not_admin = Address::generate(&env);

        let contract_id = env.register(MasterFactory, (&admin,));
        let client = MasterFactoryClient::new(&env, &contract_id);

        let dummy_wasm = BytesN::from_array(&env, &[0u8; 32]);
        let salt = BytesN::from_array(&env, &[1u8; 32]);
        client.deploy_token_factory(&not_admin, &dummy_wasm, &salt);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_deploy_nft_factory_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_master_factory(&env);
        let not_admin = Address::generate(&env);

        let dummy_wasm = BytesN::from_array(&env, &[0u8; 32]);
        let salt = BytesN::from_array(&env, &[1u8; 32]);
        client.deploy_nft_factory(&not_admin, &dummy_wasm, &salt);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_deploy_governance_factory_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_master_factory(&env);
        let not_admin = Address::generate(&env);

        let dummy_wasm = BytesN::from_array(&env, &[0u8; 32]);
        let salt = BytesN::from_array(&env, &[1u8; 32]);
        client.deploy_governance_factory(&not_admin, &dummy_wasm, &salt);
    }

    // ===== Admin Transfer Tests =====

    #[test]
    fn test_transfer_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_master_factory(&env);
        let new_admin = Address::generate(&env);

        // Two-step admin transfer
        client.initiate_admin_transfer(&current_admin, &new_admin);
        client.accept_admin_transfer(&new_admin);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, new_admin);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_transfer_admin_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_master_factory(&env);
        let not_admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        // Should panic - not admin trying to initiate transfer
        client.initiate_admin_transfer(&not_admin, &new_admin);
    }

    // ===== Upgrade Tests =====

    #[test]
    #[ignore = "Requires real WASM for upgrade - test in integration environment"]
    fn test_upgrade_requires_admin_auth() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_master_factory(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[99u8; 32]);

        // Test passes if upgrade completes successfully with proper admin auth
        // The upgrade function internally verifies admin and requires their auth
        client.upgrade(&new_wasm_hash);
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_get_admin_returns_correct_value() {
        let env = Env::default();
        let (client, admin) = setup_master_factory(&env);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, admin);
    }

    // ===== SECURITY TESTS =====
    // Note: Similar to TokenFactory security tests, adapted for MasterFactory

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // ContractPaused
    fn test_security_pause_prevents_deployments() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_master_factory(&env);

        // The pause check runs before the WASM hash is consumed, so an
        // arbitrary hash is enough: if the Paused guard were removed this
        // call would fail with a different error and the expectation below
        // would no longer match.
        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        client.pause(&admin);

        client.deploy_token_factory(&admin, &wasm_hash, &salt);
    }

    #[test]
    fn test_security_unpause_restores_functionality() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_master_factory(&env);

        // Pause then unpause
        client.pause(&admin);
        client.unpause(&admin);

        // Verify admin still works after unpause
        assert_eq!(client.get_admin(), admin);
    }

    // ===== TWO-STEP ADMIN TRANSFER TESTS =====

    #[test]
    fn test_twostep_admin_transfer_full_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_master_factory(&env);
        let new_admin = Address::generate(&env);

        // Step 1: Initiate transfer
        client.initiate_admin_transfer(&current_admin, &new_admin);

        // Verify pending admin set
        let pending = client.get_pending_admin();
        assert_eq!(pending, Some(new_admin.clone()));

        // Admin should still be current
        assert_eq!(client.get_admin(), current_admin);

        // Step 2: Accept transfer
        client.accept_admin_transfer(&new_admin);

        // Verify admin changed
        assert_eq!(client.get_admin(), new_admin);
        assert_eq!(client.get_pending_admin(), None);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // NotPendingAdmin
    fn test_twostep_wrong_acceptor() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_master_factory(&env);
        let new_admin = Address::generate(&env);
        let wrong_admin = Address::generate(&env);

        client.initiate_admin_transfer(&current_admin, &new_admin);
        client.accept_admin_transfer(&wrong_admin); // Should panic
    }

    #[test]
    fn test_twostep_cancel() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_master_factory(&env);
        let new_admin = Address::generate(&env);

        // Initiate then cancel
        client.initiate_admin_transfer(&current_admin, &new_admin);
        assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

        client.cancel_admin_transfer(&current_admin);
        assert_eq!(client.get_pending_admin(), None);
        assert_eq!(client.get_admin(), current_admin);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
    fn test_pause_requires_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_master_factory(&env);
        let not_admin = Address::generate(&env);

        client.pause(&not_admin); // Should panic
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
    fn test_unpause_requires_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_master_factory(&env);
        client.pause(&admin);

        let not_admin = Address::generate(&env);
        client.unpause(&not_admin); // Should panic
    }
}
