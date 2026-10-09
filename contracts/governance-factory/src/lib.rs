#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracterror, contracttype, panic_with_error, Address, BytesN, Env,
    IntoVal, String, Val, Vec,
};

/// GovernanceFactory - Deploys governance contracts
///
/// This contract manages deployment of governance contracts:
/// - Merkle Voting (on-chain voting with merkle proofs)
///
/// # Multisig
///
/// The `Multisig` governance type is intentionally NOT exposed by this factory
/// until a real multisig contract crate ships in this workspace. Previously a
/// caller could point `set_multisig_wasm` at an arbitrary hash and
/// `deploy_governance` would try to instantiate a template the repository never
/// ships, producing a broken/unloadable instance. The variant, its setter and
/// its deployment branch have therefore been removed. Re-introduce them
/// together with the contract crate, its WASM upload in `setup-*.sh` and
/// matching tests.
/// Multisig is intentionally not exposed: the workspace does not ship a Multisig
/// contract or a deployable WASM artifact. Restore the public variant only when
/// its implementation and deployment tooling exist.

#[contract]
pub struct GovernanceFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    MerkleVotingWasm,
    DeployedGovernance,
    MultisigWasm,
    DeployedGovernanceEntry(u32), // Indexed deployed-governance record
    GovernanceCount,
    Paused,                    // Emergency pause
    // Appended without shifting existing on-ledger variant discriminants.
    DeployedGovernanceEntry(u32),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GovernanceType {
    MerkleVoting,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceConfig {
    pub governance_type: GovernanceType,
    pub admin: Address,
    pub root_hash: Option<BytesN<32>>, // For Merkle Voting
    pub owners: Option<Vec<Address>>, // Reserved for future governance types
    pub threshold: Option<u32>,       // Reserved for future governance types
    pub salt: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceInfo {
    pub address: Address,
    pub governance_type: GovernanceType,
    pub admin: Address,
    pub timestamp: u64,
    pub name: Option<String>,
}

#[contractevent]
pub struct GovernanceDeployedEvent {
    pub governance_address: Address,
    pub governance_type: GovernanceType,
    pub deployer: Address,
    pub timestamp: u64,
}

#[contractevent]
pub struct WasmUpdatedEvent {
    pub governance_type_name: String,
    pub wasm_hash: BytesN<32>,
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
pub enum GovernanceFactoryError {
    WasmNotSet = 2,
    InvalidGovernanceType = 3,
    InvalidConfig = 4,
    CounterOverflow = 9,
}

#[contractimpl]
impl GovernanceFactory {
    /// Ledger count below which a deployed-governance record's TTL is refreshed.
    /// Approximately 30 days at ~5s per ledger.
    const RECORD_TTL_THRESHOLD: u32 = 518_400;
    /// TTL (in ledgers) a deployed-governance record is extended to.
    /// Approximately one year at ~5s per ledger.
    const RECORD_TTL_EXTEND_TO: u32 = 6_307_200;

    /// Initialize GovernanceFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        factory_common::set_admin(&e, &admin);

        // Deployed governance contracts are stored under indexed persistent
        // keys; only the small counter lives in instance storage.
        // The legacy deployed Vec is read-only on upgrade; fresh records use
        // individually addressed persistent entries.
        e.storage().instance().set(&DataKey::GovernanceCount, &0u32);

        // Initialize paused flag
        factory_common::set_paused(&e, false);
    }

    /// Set WASM hash for Merkle Voting type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Merkle Voting contract
    pub fn set_merkle_voting_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage()
            .instance()
            .set(&DataKey::MerkleVotingWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            governance_type_name: soroban_sdk::String::from_str(&e, "MerkleVoting"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Multisig type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Multisig contract
    pub fn set_multisig_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage()
            .instance()
            .set(&DataKey::MultisigWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            governance_type_name: soroban_sdk::String::from_str(&e, "Multisig"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Deploy a governance contract with specified configuration
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function
    /// * `config` - Governance configuration including type, admin, etc.
    ///
    /// # Returns
    /// Address of the deployed governance contract
    pub fn deploy_governance(e: Env, deployer: Address, config: GovernanceConfig) -> Address {
        deployer.require_auth();

        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Get WASM hash based on governance type
        let wasm_hash = Self::get_wasm_for_type(&e, &config.governance_type);

        // Validate config based on governance type
        Self::validate_config(&e, &config);

        // Deploy using deployer pattern with constructor args based on governance type
        let governance_address = match config.governance_type {
            GovernanceType::MerkleVoting => {
                // Merkle Voting requires root_hash for merkle proof verification
                let root_hash = config.root_hash.clone().unwrap_or_else(|| {
                    panic_with_error!(&e, GovernanceFactoryError::InvalidConfig)
                });
                let constructor_args: Vec<Val> = (root_hash,).into_val(&e);
                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
        };

        // Store governance info
        let governance_info = GovernanceInfo {
            address: governance_address.clone(),
            governance_type: config.governance_type.clone(),
            admin: config.admin.clone(),
            timestamp: e.ledger().timestamp(),
            name: None,
        };

        // Increment governance count with overflow protection
        let count: u32 = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0);
        let new_count = count.checked_add(1)
            .unwrap_or_else(|| {
                panic_with_error!(&e, GovernanceFactoryError::CounterOverflow)
            });

        // Store the record under its own indexed persistent key
        let index_key = DataKey::DeployedGovernanceEntry(count);
        e.storage().persistent().set(&index_key, &governance_info);
        e.storage().persistent().extend_ttl(
            &index_key,
            Self::RECORD_TTL_THRESHOLD,
            Self::RECORD_TTL_EXTEND_TO,
        );

        e.storage()
            .instance()
            .set(&DataKey::GovernanceCount, &new_count);
        // Count is the new stable index and remains a small instance entry.
        let count: u32 = e.storage().instance()
            .get(&DataKey::GovernanceCount).unwrap_or(0);
        let new_count = count.checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, GovernanceFactoryError::CounterOverflow));
        e.storage().persistent().set(&DataKey::DeployedGovernanceEntry(count), &governance_info);
        e.storage().instance().set(&DataKey::GovernanceCount, &new_count);

        // Emit event
        GovernanceDeployedEvent {
            governance_address: governance_address.clone(),
            governance_type: config.governance_type.clone(),
            deployer: deployer.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        governance_address
    }

    /// Get all deployed governance contracts
    ///
    /// # Returns
    /// Vector of GovernanceInfo containing all deployed governance contracts
    pub fn get_deployed_governance(e: Env) -> Vec<GovernanceInfo> {
        let count: u32 = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0);
        let mut governance = Vec::new(&e);
        for i in 0..count {
            if let Some(gov) = e
                .storage()
                .persistent()
                .get::<_, GovernanceInfo>(&DataKey::DeployedGovernanceEntry(i))
            {
                governance.push_back(gov);
            }
        }
        governance
    }

    /// Get a page of deployed governance contracts
    ///
    /// # Arguments
    /// * `start` - Index of the first record to return
    /// * `limit` - Maximum number of records to return
    ///
    /// # Returns
    /// Vector of GovernanceInfo for the requested page
    pub fn get_governance_paginated(e: Env, start: u32, limit: u32) -> Vec<GovernanceInfo> {
        let count: u32 = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0);
        let mut governance = Vec::new(&e);
        let mut i = start;
        while i < count && governance.len() < limit {
            if let Some(gov) = e
                .storage()
                .persistent()
                .get::<_, GovernanceInfo>(&DataKey::DeployedGovernanceEntry(i))
            {
                governance.push_back(gov);
            }
            i += 1;
        }
        governance
        let count = Self::get_governance_count(e.clone());
        Self::deployed_governance_range(&e, 0, count)
    }

    /// Read a predictable page of at most 100 deployment records.
    pub fn get_deployed_governance_page(e: Env, start: u32, limit: u32) -> Vec<GovernanceInfo> {
        let count = Self::get_governance_count(e.clone());
        let end = start.saturating_add(limit.min(100)).min(count);
        Self::deployed_governance_range(&e, start, end)
    }

    fn deployed_governance_range(e: &Env, start: u32, end: u32) -> Vec<GovernanceInfo> {
        let mut result = Vec::new(e);
        let legacy: Vec<GovernanceInfo> = e.storage().instance()
            .get(&DataKey::DeployedGovernance).unwrap_or_else(|| Vec::new(e));
        for index in start..end {
            let indexed: Option<GovernanceInfo> = e.storage().persistent()
                .get(&DataKey::DeployedGovernanceEntry(index));
            if let Some(info) = indexed.or_else(|| legacy.get(index)) {
                result.push_back(info);
            }
        }
        result
    }

    /// Get governance contracts by type
    ///
    /// # Arguments
    /// * `governance_type` - Type of governance to filter by
    ///
    /// # Returns
    /// Vector of GovernanceInfo for the specified type
    pub fn get_governance_by_type(e: Env, governance_type: GovernanceType) -> Vec<GovernanceInfo> {
        let count: u32 = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0);
        let all_governance = Self::get_deployed_governance(e.clone());

        let mut filtered = Vec::new(&e);
        for i in 0..count {
            if let Some(gov) = e
                .storage()
                .persistent()
                .get::<_, GovernanceInfo>(&DataKey::DeployedGovernanceEntry(i))
            {
                if gov.governance_type == governance_type {
                    filtered.push_back(gov);
                }
            }
        }
        filtered
    }

    /// Get governance contracts by admin
    ///
    /// # Arguments
    /// * `admin` - Admin address to filter by
    ///
    /// # Returns
    /// Vector of GovernanceInfo for contracts managed by the admin
    pub fn get_governance_by_admin(e: Env, admin: Address) -> Vec<GovernanceInfo> {
        let count: u32 = e
            .storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0);
        let all_governance = Self::get_deployed_governance(e.clone());

        let mut filtered = Vec::new(&e);
        for i in 0..count {
            if let Some(gov) = e
                .storage()
                .persistent()
                .get::<_, GovernanceInfo>(&DataKey::DeployedGovernanceEntry(i))
            {
                if gov.admin == admin {
                    filtered.push_back(gov);
                }
            }
        }
        filtered
    }

    /// Get total number of deployed governance contracts
    ///
    /// # Returns
    /// Total count of deployed governance contracts
    pub fn get_governance_count(e: Env) -> u32 {
        e.storage()
            .instance()
            .get(&DataKey::GovernanceCount)
            .unwrap_or(0)
    }

    /// Upgrade the factory contract to a new WASM hash
    ///
    /// # Arguments
    /// * `new_wasm_hash` - New WASM hash to upgrade to
    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        // Get admin and require their authorization
        let admin = factory_common::get_admin(&e);
        admin.require_auth();

        // Pause contract during upgrade for safety
        factory_common::set_paused(&e, true);

        // Emit upgrade event
        ContractUpgradedEvent {
            new_wasm_hash: new_wasm_hash.clone(),
        }
        .publish(&e);

        e.deployer().update_current_contract_wasm(new_wasm_hash);

        // Note: Contract will be paused after upgrade, admin must unpause
    }

    // Helper: Get WASM hash for governance type
    fn get_wasm_for_type(e: &Env, governance_type: &GovernanceType) -> BytesN<32> {
        let key = match governance_type {
            GovernanceType::MerkleVoting => DataKey::MerkleVotingWasm,
        };

        e.storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(e, GovernanceFactoryError::WasmNotSet))
    }

    // Helper: Validate governance configuration
    fn validate_config(e: &Env, config: &GovernanceConfig) {
        match config.governance_type {
            GovernanceType::MerkleVoting => {
                // Merkle Voting must have root_hash
                if config.root_hash.is_none() {
                    panic_with_error!(e, GovernanceFactoryError::InvalidConfig);
                }
            }
        }
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

    /// Get pending admin address
    ///
    /// # Returns
    /// Optional pending admin address
    pub fn get_pending_admin(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::PendingAdmin)
    }

    // Helper: Get WASM hash for governance type
    fn get_wasm_for_type(e: &Env, governance_type: &GovernanceType) -> BytesN<32> {
        let key = match governance_type {
            GovernanceType::MerkleVoting => DataKey::MerkleVotingWasm,
        };

        e.storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(e, GovernanceFactoryError::WasmNotSet))
    }

    // Helper: Validate Merkle Voting configuration.
    fn validate_config(e: &Env, config: &GovernanceConfig) {
        if config.root_hash.is_none() {
            panic_with_error!(e, GovernanceFactoryError::InvalidConfig);
        }
    }

    // Helper: Check admin authorization
    fn require_admin(e: &Env, address: &Address) {
        let admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(e, GovernanceFactoryError::AdminNotSet));
        if admin != *address {
            panic_with_error!(e, GovernanceFactoryError::NotAdmin);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup_governance_factory(env: &Env) -> (GovernanceFactoryClient, Address) {
        let admin = Address::generate(env);
        let contract_id = env.register(GovernanceFactory, (&admin,));
        let client = GovernanceFactoryClient::new(env, &contract_id);
        (client, admin)
    }

    fn setup_with_wasm(env: &Env) -> (GovernanceFactoryClient, Address, BytesN<32>) {
        env.mock_all_auths();
        let (client, admin) = setup_governance_factory(env);
        let wasm_hash = BytesN::from_array(env, &[1u8; 32]);

        client.set_merkle_voting_wasm(&admin, &wasm_hash);

        (client, admin, wasm_hash)
    }

    // ===== Constructor Tests =====

    #[test]
    fn test_constructor() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(GovernanceFactory, (&admin,));
        let client = GovernanceFactoryClient::new(&env, &contract_id);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);

        let count = client.get_governance_count();
        assert_eq!(count, 0);

        let governance = client.get_deployed_governance();
        assert_eq!(governance.len(), 0);
    }

    // ===== WASM Configuration Tests =====

    #[test]
    fn test_set_wasm_hashes() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(GovernanceFactory, (&admin,));
        let client = GovernanceFactoryClient::new(&env, &contract_id);

        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // Should not panic
        client.set_merkle_voting_wasm(&admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_merkle_voting_wasm_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_governance_factory(&env);
        let not_admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        client.set_merkle_voting_wasm(&not_admin, &wasm_hash);
    }

    // ===== Validation Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_merkle_voting_missing_root_hash() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin,
            root_hash: None, // Missing
            governance_type: GovernanceType::Multisig,
            admin,
            root_hash: None,
            owners: None, // Missing
            threshold: Some(2),
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_multisig_missing_threshold() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let mut owners = Vec::new(&env);
        owners.push_back(owner1);
        owners.push_back(owner2);

        let config = GovernanceConfig {
            governance_type: GovernanceType::Multisig,
            admin,
            root_hash: None,
            owners: Some(owners),
            threshold: None, // Missing
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_multisig_threshold_zero() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let mut owners = Vec::new(&env);
        owners.push_back(owner1);
        owners.push_back(owner2);

        let config = GovernanceConfig {
            governance_type: GovernanceType::Multisig,
            admin,
            root_hash: None,
            owners: Some(owners),
            threshold: Some(0), // Invalid: 0
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_multisig_threshold_too_high() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let mut owners = Vec::new(&env);
        owners.push_back(owner1);
        owners.push_back(owner2);

        let config = GovernanceConfig {
            governance_type: GovernanceType::Multisig,
            admin,
            root_hash: None,
            owners: Some(owners),
            threshold: Some(3), // Invalid: > owners.len()
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_deploy_governance_wasm_not_set() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_governance_factory(&env);
        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin,
            root_hash: None,
            root_hash: Some(BytesN::from_array(&env, &[2u8; 32])),
            owners: None,
            threshold: None,
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    // ===== Query Tests =====

    #[test]
    fn test_get_deployed_governance_empty() {
        let env = Env::default();
        let (client, _admin) = setup_governance_factory(&env);

        let governance = client.get_deployed_governance();
        assert_eq!(governance.len(), 0);
    }

    #[test]
    fn test_get_governance_by_type_empty() {
        let env = Env::default();
        let (client, _admin) = setup_governance_factory(&env);

        let governance = client.get_governance_by_type(&GovernanceType::MerkleVoting);
        assert_eq!(governance.len(), 0);
    }

    #[test]
    fn test_get_governance_by_admin_empty() {
        let env = Env::default();
        let (client, _admin) = setup_governance_factory(&env);
        let admin = Address::generate(&env);

        let governance = client.get_governance_by_admin(&admin);
        assert_eq!(governance.len(), 0);
    }

    #[test]
    fn test_get_governance_count() {
        let env = Env::default();
        let (client, _admin) = setup_governance_factory(&env);

        let count = client.get_governance_count();
        assert_eq!(count, 0);
    }

    // ===== Admin Transfer Tests =====

    #[test]
    fn test_transfer_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_governance_factory(&env);
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

        let (client, _admin) = setup_governance_factory(&env);
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

        let (client, _admin) = setup_governance_factory(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[99u8; 32]);

        // Test passes if upgrade completes successfully with proper admin auth
        // The upgrade function internally verifies admin and requires their auth
        client.upgrade(&new_wasm_hash);
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_get_admin_returns_correct_value() {
        let env = Env::default();
        let (client, admin) = setup_governance_factory(&env);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, admin);
    }

    #[test]
    fn test_multiple_admin_transfers() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin1) = setup_governance_factory(&env);
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);

        // Transfer to admin2
        client.initiate_admin_transfer(&admin1, &admin2);
        client.accept_admin_transfer(&admin2);
        assert_eq!(client.get_admin(), admin2);

        // Transfer to admin3
        client.initiate_admin_transfer(&admin2, &admin3);
        client.accept_admin_transfer(&admin3);
        assert_eq!(client.get_admin(), admin3);
    }

    // ===== Pause/Unpause Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_deploy_governance_panics_while_paused() {
        let env = Env::default();
        let (client, admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin: admin.clone(),
            root_hash: Some(BytesN::from_array(&env, &[9u8; 32])),
            owners: None,
            threshold: None,
            salt: BytesN::from_array(&env, &[3u8; 32]),
        };

        client.pause(&admin);
        client.deploy_governance(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_governance_passes_pause_gate_after_unpause() {
        let env = Env::default();
        let (client, admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        // root_hash: None for MerkleVoting -> InvalidConfig (#4). Reaching #4
        // proves the call passed the paused check; while paused it fails at #8.
        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin: admin.clone(),
            root_hash: None,
            owners: None,
            threshold: None,
            salt: BytesN::from_array(&env, &[4u8; 32]),
        };

        client.pause(&admin);
        client.unpause(&admin);
        client.deploy_governance(&deployer, &config);
    }
    // ===== Pause / Unpause Tests =====

    /// While the factory is paused, `deploy_governance` is rejected before any
    /// deployment work happens, bubbling up
    /// `GovernanceFactoryError::ContractPaused` (`Contract, #8`).
    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_pause_blocks_deploy_governance() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, _wasm) = setup_with_wasm(&env);
        client.pause(&admin);

        let deployer = Address::generate(&env);
        let gov_admin = Address::generate(&env);
        let root_hash = BytesN::from_array(&env, &[3u8; 32]);
        let salt = BytesN::from_array(&env, &[7u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin: gov_admin,
            root_hash: Some(root_hash),
    /// The Merkle Voting branch requires a `root_hash`; without it the deploy
    /// is rejected with `GovernanceFactoryError::InvalidConfig` (`Contract, #4`).
    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_merkle_voting_missing_root_hash() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[9u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin,
            root_hash: None, // Missing
            owners: None,
            threshold: None,
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

    /// After `unpause`, the pause gate is lifted: `deploy_governance` now gets
    /// past the `Paused` check and fails further down at the (unconfigured)
    /// WASM lookup, `GovernanceFactoryError::WasmNotSet` (`Contract, #2`).
    ///
    /// NOTE: a genuine *successful* deployment cannot be asserted here with a
    /// plain `cargo test`: `deploy_v2` requires an uploaded contract WASM,
    /// which unit tests can only obtain from a compiled `.wasm` artifact
    /// (via `contractimport!`) that this workspace does not build. Reaching
    /// `WasmNotSet` proves the `ContractPaused` branch was skipped.
    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_unpause_allows_deploy_governance_to_proceed() {
        let env = Env::default();
        env.mock_all_auths();

        // Deliberately no WASM configured, so the only gate left to observe is
        // the pause flag itself.
        let (client, admin) = setup_governance_factory(&env);
        client.pause(&admin);
        client.unpause(&admin);

        let deployer = Address::generate(&env);
        let gov_admin = Address::generate(&env);
        let root_hash = BytesN::from_array(&env, &[4u8; 32]);
        let salt = BytesN::from_array(&env, &[8u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin: gov_admin,
    /// Exercises the Merkle Voting deployment branch with a `root_hash`: after
    /// config validation passes, the constructor tuple `(root_hash,)` is built
    /// and `deploy_v2` is reached. With a plain `cargo test` run the configured
    /// WASM hash is not a real uploaded artifact, so the host rejects the deploy
    /// and nothing is recorded.
    ///
    /// NOTE: asserting a *successful* Merkle Voting deployment (and the recorded
    /// `GovernanceInfo`) requires a compiled contract WASM
    /// (`contractimport!`/`stellar contract build`), which this workspace does
    /// not build for unit tests; this test pins the reachable behaviour up to
    /// that host boundary.
    #[test]
    fn test_deploy_merkle_voting_with_root_hash_reaches_deploy_v2() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[10u8; 32]);
        let root_hash = BytesN::from_array(&env, &[3u8; 32]);

        let config = GovernanceConfig {
            governance_type: GovernanceType::MerkleVoting,
            admin,
            root_hash: Some(root_hash),
            owners: None,
            threshold: None,
            salt,
        };

        client.deploy_governance(&deployer, &config);
    }

        // Reaches `deploy_v2` (config validation passed); the placeholder WASM
        // hash is not a real uploaded module, so the host fails the deploy.
        let result = client.try_deploy_governance(&deployer, &config);
        assert!(result.is_err());

        // A failed deployment records nothing.
        assert_eq!(client.get_governance_count(), 0);
        assert_eq!(
            client.get_governance_by_type(&GovernanceType::MerkleVoting).len(),
            0
        );
    }
}
