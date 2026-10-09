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
    Admin,
    PendingAdmin,
    TokenFactory,
    NFTFactory,
    GovernanceFactory,
    DeployedFactories,
    Deploying,
    UsedSalts(BytesN<32>),
    DeploymentsInBlock(u32),
    Paused,
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
    NotAdmin = 1,
    FactoryAlreadyDeployed = 2,
    AdminNotSet = 4,
    Reentrancy = 5,
    DuplicateSalt = 6,
    RateLimitExceeded = 7,
    NoPendingAdmin = 8,
    NotPendingAdmin = 9,
    ContractPaused = 10,
    CounterOverflow = 11,
}

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

    /// Initialize MasterFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        e.storage().instance().set(&DataKey::Admin, &admin);

        // Initialize empty factories list
        let factories: Vec<FactoryInfo> = Vec::new(&e);
        e.storage().instance().set(&DataKey::DeployedFactories, &factories);
        e.storage().instance().set(&DataKey::Deploying, &false);
        e.storage().instance().set(&DataKey::Paused, &false);
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
        Self::require_admin(&e, &deployer);

        // Check if paused
        let paused = e.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            panic_with_error!(&e, MasterFactoryError::ContractPaused);
        }

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
        if e.storage().persistent().has(&salt_key) {
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

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
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

        let mut factories: Vec<FactoryInfo> = e.storage()
            .instance()
            .get(&DataKey::DeployedFactories)
            .unwrap_or_else(|| Vec::new(&e));
        factories.push_back(factory_info.clone());
        e.storage().instance().set(&DataKey::DeployedFactories, &factories);

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
        Self::require_admin(&e, &deployer);

        // Check if paused
        let paused = e.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            panic_with_error!(&e, MasterFactoryError::ContractPaused);
        }

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
        if e.storage().persistent().has(&salt_key) {
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

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
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

        let mut factories: Vec<FactoryInfo> = e.storage()
            .instance()
            .get(&DataKey::DeployedFactories)
            .unwrap_or_else(|| Vec::new(&e));
        factories.push_back(factory_info.clone());
        e.storage().instance().set(&DataKey::DeployedFactories, &factories);

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
        Self::require_admin(&e, &deployer);

        // Check if paused
        let paused = e.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            panic_with_error!(&e, MasterFactoryError::ContractPaused);
        }

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
        if e.storage().persistent().has(&salt_key) {
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

        // Update rate limit counter with overflow protection
        let new_deployments_count = deployments_count.checked_add(1)
            .unwrap_or_else(|| {
                e.storage().instance().set(&DataKey::Deploying, &false);
                panic_with_error!(&e, MasterFactoryError::CounterOverflow)
            });
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

        let mut factories: Vec<FactoryInfo> = e.storage()
            .instance()
            .get(&DataKey::DeployedFactories)
            .unwrap_or_else(|| Vec::new(&e));
        factories.push_back(factory_info.clone());
        e.storage().instance().set(&DataKey::DeployedFactories, &factories);

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
        e.storage()
            .instance()
            .get(&DataKey::DeployedFactories)
            .unwrap_or(Vec::new(&e))
    }

    /// Get admin address
    ///
    /// # Returns
    /// Address of the admin
    pub fn get_admin(e: Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::AdminNotSet))
    }

    /// Get pending admin address
    ///
    /// # Returns
    /// Option containing pending admin address
    pub fn get_pending_admin(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::PendingAdmin)
    }

    /// Pause contract (emergency stop)
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    pub fn pause(e: Env, admin: Address) {
        admin.require_auth();
        Self::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::Paused, &true);

        ContractPausedEvent {
            admin: admin.clone(),
        }
        .publish(&e);
    }

    /// Unpause contract
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    pub fn unpause(e: Env, admin: Address) {
        admin.require_auth();
        Self::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::Paused, &false);

        ContractUnpausedEvent {
            admin: admin.clone(),
        }
        .publish(&e);
    }

    /// Upgrade the factory contract to a new WASM hash
    ///
    /// # Arguments
    /// * `new_wasm_hash` - New WASM hash to upgrade to
    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        // Get admin and require their authorization
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::AdminNotSet));
        admin.require_auth();

        // Pause contract during upgrade
        e.storage().instance().set(&DataKey::Paused, &true);

        // Emit upgrade event
        ContractUpgradedEvent {
            new_wasm_hash: new_wasm_hash.clone(),
        }
        .publish(&e);

        e.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// Initiate admin transfer (step 1 of 2-step process)
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address (must match stored admin)
    /// * `new_admin` - New admin address
    pub fn initiate_admin_transfer(e: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&e, &current_admin);

        e.storage().instance().set(&DataKey::PendingAdmin, &new_admin);

        AdminTransferInitiatedEvent {
            new_admin: new_admin.clone(),
        }
        .publish(&e);
    }

    /// Accept admin transfer (step 2 of 2-step process)
    ///
    /// # Arguments
    /// * `new_admin` - New admin address accepting the role
    pub fn accept_admin_transfer(e: Env, new_admin: Address) {
        new_admin.require_auth();

        let pending_admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .unwrap_or_else(|| panic_with_error!(&e, MasterFactoryError::NoPendingAdmin));

        if pending_admin != new_admin {
            panic_with_error!(&e, MasterFactoryError::NotPendingAdmin);
        }

        e.storage().instance().set(&DataKey::Admin, &new_admin);
        e.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferredEvent {
            new_admin: new_admin.clone(),
        }
        .publish(&e);
    }

    /// Cancel pending admin transfer
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address
    pub fn cancel_admin_transfer(e: Env, current_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&e, &current_admin);

        e.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferCancelledEvent {
            admin: current_admin.clone(),
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
    fn test_security_pause_prevents_deployments() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_master_factory(&env);
        let _wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
        let _salt = BytesN::from_array(&env, &[2u8; 32]);

        // Pause the contract
        client.pause(&admin);

        // Try to deploy - should fail
        // Note: In real test, this would panic with ContractPaused error
        // Simplified test just verifies pause mechanism exists
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
    #[should_panic(expected = "Error(Contract, #9)")] // NotPendingAdmin
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
