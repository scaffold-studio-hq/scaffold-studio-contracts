#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracterror, contracttype, panic_with_error, Address, BytesN, Env,
    IntoVal, String, Val, Vec,
};

/// GovernanceFactory - Deploys governance contracts
///
/// This contract manages deployment of governance contracts:
/// - Merkle Voting (on-chain voting with merkle proofs)
/// Multisig is intentionally not exposed: the workspace does not ship a Multisig
/// contract or a deployable WASM artifact. Restore the public variant only when
/// its implementation and deployment tooling exist.

#[contract]
pub struct GovernanceFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    PendingAdmin,              // Two-step admin transfer
    MerkleVotingWasm,
    DeployedGovernance,
    GovernanceCount,
    Paused,                    // Emergency pause
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
    NotAdmin = 1,
    WasmNotSet = 2,
    InvalidGovernanceType = 3,
    InvalidConfig = 4,
    AdminNotSet = 5,
    NoPendingAdmin = 6,
    NotPendingAdmin = 7,
    ContractPaused = 8,
    CounterOverflow = 9,
}

#[contractimpl]
impl GovernanceFactory {
    /// Initialize GovernanceFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        e.storage().instance().set(&DataKey::Admin, &admin);

        // Initialize empty governance list
        let governance: Vec<GovernanceInfo> = Vec::new(&e);
        e.storage()
            .instance()
            .set(&DataKey::DeployedGovernance, &governance);
        e.storage().instance().set(&DataKey::GovernanceCount, &0u32);

        // Initialize paused flag
        e.storage().instance().set(&DataKey::Paused, &false);
    }

    /// Set WASM hash for Merkle Voting type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Merkle Voting contract
    pub fn set_merkle_voting_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        Self::require_admin(&e, &admin);
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

        // Check if paused
        let paused = e.storage().instance().get(&DataKey::Paused).unwrap_or(false);
        if paused {
            panic_with_error!(&e, GovernanceFactoryError::ContractPaused);
        }

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

        let mut governance: Vec<GovernanceInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedGovernance)
            .unwrap_or_else(|| Vec::new(&e));
        governance.push_back(governance_info);
        e.storage()
            .instance()
            .set(&DataKey::DeployedGovernance, &governance);

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
        e.storage()
            .instance()
            .set(&DataKey::GovernanceCount, &new_count);

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
        e.storage()
            .instance()
            .get(&DataKey::DeployedGovernance)
            .unwrap_or(Vec::new(&e))
    }

    /// Get governance contracts by type
    ///
    /// # Arguments
    /// * `governance_type` - Type of governance to filter by
    ///
    /// # Returns
    /// Vector of GovernanceInfo for the specified type
    pub fn get_governance_by_type(e: Env, governance_type: GovernanceType) -> Vec<GovernanceInfo> {
        let all_governance: Vec<GovernanceInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedGovernance)
            .unwrap_or(Vec::new(&e));

        let mut filtered = Vec::new(&e);
        for gov in all_governance.iter() {
            if gov.governance_type == governance_type {
                filtered.push_back(gov);
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
        let all_governance: Vec<GovernanceInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedGovernance)
            .unwrap_or(Vec::new(&e));

        let mut filtered = Vec::new(&e);
        for gov in all_governance.iter() {
            if gov.admin == admin {
                filtered.push_back(gov);
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

    /// Get admin address
    ///
    /// # Returns
    /// Address of the admin
    pub fn get_admin(e: Env) -> Address {
        e.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&e, GovernanceFactoryError::AdminNotSet))
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
            .unwrap_or_else(|| panic_with_error!(&e, GovernanceFactoryError::AdminNotSet));
        admin.require_auth();

        // Pause contract during upgrade for safety
        e.storage().instance().set(&DataKey::Paused, &true);

        // Emit upgrade event
        ContractUpgradedEvent {
            new_wasm_hash: new_wasm_hash.clone(),
        }
        .publish(&e);

        e.deployer().update_current_contract_wasm(new_wasm_hash);

        // Note: Contract will be paused after upgrade, admin must unpause
    }

    /// Pause the contract (emergency stop)
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

    /// Unpause the contract
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

    /// Initiate admin transfer (step 1 of 2)
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address (must match stored admin)
    /// * `new_admin` - New admin address to transfer to
    pub fn initiate_admin_transfer(e: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&e, &current_admin);

        e.storage().instance().set(&DataKey::PendingAdmin, &new_admin);

        AdminTransferInitiatedEvent {
            new_admin: new_admin.clone(),
        }
        .publish(&e);
    }

    /// Accept admin transfer (step 2 of 2)
    ///
    /// # Arguments
    /// * `new_admin` - New admin address (must match pending admin)
    pub fn accept_admin_transfer(e: Env, new_admin: Address) {
        new_admin.require_auth();

        let pending_admin: Address = e
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .unwrap_or_else(|| panic_with_error!(&e, GovernanceFactoryError::NoPendingAdmin));

        if pending_admin != new_admin {
            panic_with_error!(&e, GovernanceFactoryError::NotPendingAdmin);
        }

        e.storage().instance().set(&DataKey::Admin, &new_admin);
        e.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferredEvent {
            new_admin: new_admin.clone(),
        }
        .publish(&e);
    }

    /// Cancel admin transfer
    ///
    /// # Arguments
    /// * `current_admin` - Current admin address (for authorization)
    pub fn cancel_admin_transfer(e: Env, current_admin: Address) {
        current_admin.require_auth();
        Self::require_admin(&e, &current_admin);

        e.storage().instance().remove(&DataKey::PendingAdmin);

        AdminTransferCancelledEvent {
            admin: current_admin.clone(),
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
}
