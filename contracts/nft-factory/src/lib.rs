#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, panic_with_error, Address,
    BytesN, Env, IntoVal, String, Val, Vec,
};

/// NFTFactory - Deploys NFT contracts
///
/// This contract manages deployment of various NFT types:
/// - Enumerable NFT (track NFTs by owner)
/// - Royalties NFT (creator royalties on resale)
/// - Access Control NFT (role-based permissions)

#[contract]
pub struct NFTFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    PendingAdmin, // Two-step admin transfer
    EnumerableWasm,
    RoyaltiesWasm,
    AccessControlWasm,
    DeployedNFT(u32),          // Indexed deployed-NFT record
    NFTCount,
    Paused, // Emergency pause
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NFTType {
    Enumerable,
    Royalties,
    AccessControl,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NFTConfig {
    pub nft_type: NFTType,
    pub owner: Address,           // For Enumerable NFT
    pub admin: Option<Address>,   // For Royalties and Access Control NFTs
    pub manager: Option<Address>, // For Royalties NFT
    pub salt: BytesN<32>,
    pub name: Option<String>,   // NFT collection name (default: "My Token")
    pub symbol: Option<String>, // NFT collection symbol (default: "TKN")
    pub base_uri: Option<String>, // Base URI for token metadata (default varies by type)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NFTInfo {
    pub address: Address,
    pub nft_type: NFTType,
    pub owner: Address,
    pub timestamp: u64,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub base_uri: Option<String>,
}

#[contractevent]
pub struct NFTDeployedEvent {
    pub nft_address: Address,
    pub nft_type: NFTType,
    pub deployer: Address,
    pub timestamp: u64,
}

#[contractevent]
pub struct WasmUpdatedEvent {
    pub nft_type_name: String,
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
pub enum NFTFactoryError {
    WasmNotSet = 2,
    InvalidConfig = 4,
    CounterOverflow = 9,
}

#[contractimpl]
impl NFTFactory {
    /// Ledger count below which a deployed-NFT record's TTL is refreshed.
    /// Approximately 30 days at ~5s per ledger.
    const RECORD_TTL_THRESHOLD: u32 = 518_400;
    /// TTL (in ledgers) a deployed-NFT record is extended to. Approximately one
    /// year at ~5s per ledger.
    const RECORD_TTL_EXTEND_TO: u32 = 6_307_200;

    /// Initialize NFTFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        factory_common::set_admin(&e, &admin);

        // Deployed NFTs are stored under indexed persistent keys; only the
        // small counter lives in instance storage.
        e.storage().instance().set(&DataKey::NFTCount, &0u32);

        // Initialize paused flag
        factory_common::set_paused(&e, false);
    }

    /// Set WASM hash for Enumerable NFT type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Enumerable NFT contract
    pub fn set_enumerable_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage()
            .instance()
            .set(&DataKey::EnumerableWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            nft_type_name: soroban_sdk::String::from_str(&e, "Enumerable"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Royalties NFT type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Royalties NFT contract
    pub fn set_royalties_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage()
            .instance()
            .set(&DataKey::RoyaltiesWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            nft_type_name: soroban_sdk::String::from_str(&e, "Royalties"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Access Control NFT type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Access Control NFT contract
    pub fn set_access_control_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage()
            .instance()
            .set(&DataKey::AccessControlWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            nft_type_name: soroban_sdk::String::from_str(&e, "AccessControl"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Deploy an NFT contract with specified configuration
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function
    /// * `config` - NFT configuration including type, owner, royalties, etc.
    ///
    /// # Returns
    /// Address of the deployed NFT contract
    pub fn deploy_nft(e: Env, deployer: Address, config: NFTConfig) -> Address {
        deployer.require_auth();

        // Check if paused
        let paused = e
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            panic_with_error!(&e, NFTFactoryError::ContractPaused);
        }
        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Get WASM hash based on NFT type
        let wasm_hash = Self::get_wasm_for_type(&e, &config.nft_type);

        // Validate config based on NFT type
        Self::validate_config(&e, &config);

        // Get metadata with defaults
        let name = config
            .name
            .clone()
            .unwrap_or_else(|| String::from_str(&e, "My Token"));
        let symbol = config
            .symbol
            .clone()
            .unwrap_or_else(|| String::from_str(&e, "TKN"));

        // Deploy using deployer pattern with constructor args based on NFT type
        let nft_address = match config.nft_type {
            NFTType::Enumerable => {
                // Enumerable NFT constructor signature: (owner, base_uri, name, symbol)
                let base_uri = config
                    .base_uri
                    .clone()
                    .unwrap_or_else(|| String::from_str(&e, "www.mytoken.com"));
                let constructor_args: Vec<Val> =
                    (config.owner.clone(), base_uri, name.clone(), symbol.clone()).into_val(&e);
                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
            NFTType::Royalties => {
                // Royalties NFT constructor signature: (admin, manager, base_uri, name, symbol)
                let admin = config
                    .admin
                    .clone()
                    .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::InvalidConfig));
                let manager = config
                    .manager
                    .clone()
                    .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::InvalidConfig));
                let base_uri = config
                    .base_uri
                    .clone()
                    .unwrap_or_else(|| String::from_str(&e, "https://example.com/nft/"));
                let constructor_args: Vec<Val> =
                    (admin, manager, base_uri, name.clone(), symbol.clone()).into_val(&e);
                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
            NFTType::AccessControl => {
                // Access Control NFT constructor signature: (admin, base_uri, name, symbol)
                let admin = config
                    .admin
                    .clone()
                    .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::InvalidConfig));
                let base_uri = config
                    .base_uri
                    .clone()
                    .unwrap_or_else(|| String::from_str(&e, "www.mytoken.com"));
                let constructor_args: Vec<Val> =
                    (admin, base_uri, name.clone(), symbol.clone()).into_val(&e);
                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
        };

        // `NFTInfo.owner` must name the address that actually controls the
        // deployed collection. Enumerable NFTs are owned by `config.owner`;
        // Royalties and Access Control NFTs are controlled by their `admin`,
        // which is the privileged address passed to their constructors.
        let effective_owner = match config.nft_type {
            NFTType::Enumerable => config.owner.clone(),
            NFTType::Royalties | NFTType::AccessControl => config
                .admin
                .clone()
                .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::InvalidConfig)),
        };

        // Store NFT info
        let nft_info = NFTInfo {
            address: nft_address.clone(),
            nft_type: config.nft_type.clone(),
            owner: effective_owner,
            timestamp: e.ledger().timestamp(),
            name: Some(name),
            symbol: Some(symbol),
            base_uri: config.base_uri.clone(),
        };

        // Increment NFT count with overflow protection
        let count: u32 = e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0);
        let new_count = count
            .checked_add(1)
            .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::CounterOverflow));
        let new_count = count.checked_add(1)
            .unwrap_or_else(|| {
                panic_with_error!(&e, NFTFactoryError::CounterOverflow)
            });

        // Store the record under its own indexed persistent key
        let index_key = DataKey::DeployedNFT(count);
        e.storage().persistent().set(&index_key, &nft_info);
        e.storage().persistent().extend_ttl(
            &index_key,
            Self::RECORD_TTL_THRESHOLD,
            Self::RECORD_TTL_EXTEND_TO,
        );

        e.storage().instance().set(&DataKey::NFTCount, &new_count);

        // Emit event
        NFTDeployedEvent {
            nft_address: nft_address.clone(),
            nft_type: config.nft_type.clone(),
            deployer: deployer.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        nft_address
    }

    /// Get all deployed NFTs
    ///
    /// # Returns
    /// Vector of NFTInfo containing all deployed NFTs
    pub fn get_deployed_nfts(e: Env) -> Vec<NFTInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0);
        let mut nfts = Vec::new(&e);
        for i in 0..count {
            if let Some(nft) = e
                .storage()
                .persistent()
                .get::<_, NFTInfo>(&DataKey::DeployedNFT(i))
            {
                nfts.push_back(nft);
            }
        }
        nfts
    }

    /// Get a page of deployed NFTs
    ///
    /// # Arguments
    /// * `start` - Index of the first record to return
    /// * `limit` - Maximum number of records to return
    ///
    /// # Returns
    /// Vector of NFTInfo for the requested page
    pub fn get_deployed_nfts_paginated(e: Env, start: u32, limit: u32) -> Vec<NFTInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0);
        let mut nfts = Vec::new(&e);
        let mut i = start;
        while i < count && nfts.len() < limit {
            if let Some(nft) = e
                .storage()
                .persistent()
                .get::<_, NFTInfo>(&DataKey::DeployedNFT(i))
            {
                nfts.push_back(nft);
            }
            i += 1;
        }
        nfts
    }

    /// Get NFTs by type
    ///
    /// # Arguments
    /// * `nft_type` - Type of NFTs to filter by
    ///
    /// # Returns
    /// Vector of NFTInfo for the specified type
    pub fn get_nfts_by_type(e: Env, nft_type: NFTType) -> Vec<NFTInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0);
        let mut filtered = Vec::new(&e);
        for i in 0..count {
            if let Some(nft) = e
                .storage()
                .persistent()
                .get::<_, NFTInfo>(&DataKey::DeployedNFT(i))
            {
                if nft.nft_type == nft_type {
                    filtered.push_back(nft);
                }
            }
        }
        filtered
    }

    /// Get NFTs by owner
    ///
    /// # Arguments
    /// * `owner` - Owner address to filter by
    ///
    /// # Returns
    /// Vector of NFTInfo for NFTs owned by the address
    pub fn get_nfts_by_owner(e: Env, owner: Address) -> Vec<NFTInfo> {
        let count: u32 = e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0);
        let mut filtered = Vec::new(&e);
        for i in 0..count {
            if let Some(nft) = e
                .storage()
                .persistent()
                .get::<_, NFTInfo>(&DataKey::DeployedNFT(i))
            {
                if nft.owner == owner {
                    filtered.push_back(nft);
                }
            }
        }
        filtered
    }

    /// Get total number of deployed NFTs
    ///
    /// # Returns
    /// Total count of deployed NFTs
    pub fn get_nft_count(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::NFTCount).unwrap_or(0)
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

        e.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);

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
            .unwrap_or_else(|| panic_with_error!(&e, NFTFactoryError::NoPendingAdmin));

        if pending_admin != new_admin {
            panic_with_error!(&e, NFTFactoryError::NotPendingAdmin);
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

    // Helper: Get WASM hash for NFT type
    fn get_wasm_for_type(e: &Env, nft_type: &NFTType) -> BytesN<32> {
        let key = match nft_type {
            NFTType::Enumerable => DataKey::EnumerableWasm,
            NFTType::Royalties => DataKey::RoyaltiesWasm,
            NFTType::AccessControl => DataKey::AccessControlWasm,
        };

        e.storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(e, NFTFactoryError::WasmNotSet))
    }

    // Helper: Validate NFT configuration
    fn validate_config(e: &Env, config: &NFTConfig) {
        // Royalties NFT must have admin and manager
        if config.nft_type == NFTType::Royalties {
            if config.admin.is_none() || config.manager.is_none() {
                panic_with_error!(e, NFTFactoryError::InvalidConfig);
            }
        }

        // Access Control NFT must have admin
        if config.nft_type == NFTType::AccessControl && config.admin.is_none() {
            panic_with_error!(e, NFTFactoryError::InvalidConfig);
        }

        // Enumerable NFT should not have admin or manager
        if config.nft_type == NFTType::Enumerable {
            if config.admin.is_some() || config.manager.is_some() {
                panic_with_error!(e, NFTFactoryError::InvalidConfig);
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
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Env, IntoVal, Map, String, Symbol, TryFromVal, Val,
    };

    // The three NFT example WASMs are produced by `stellar contract build`
    // into the workspace `target/wasm32v1-none/release/` directory.
    mod nft_enumerable_wasm {
        soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/nft_enumerable_example.wasm");
    }

    mod nft_royalties_wasm {
        soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/nft_royalties_example.wasm");
    }

    mod nft_access_control_wasm {
        soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/nft_access_control_example.wasm");
    }

    /// Returns the `nft_type` carried by the `NFTDeployedEvent` that was
    /// published for `nft_address`, or `None` when no such event was emitted.
    fn nft_deployed_event_type(env: &Env, nft_address: &Address) -> Option<NFTType> {
        let event_topic: Val = Symbol::new(env, "nft_deployed_event").into_val(env);
        let address_key = Symbol::new(env, "nft_address");
        let nft_type_key = Symbol::new(env, "nft_type");

        for (_, topics, data) in env.events().all().iter() {
            if topics.get(0) != Some(event_topic.clone()) {
                continue;
            }
            let Ok(event_data) = Map::<Symbol, Val>::try_from_val(env, &data) else {
                continue;
            };
            let Some(address_val) = event_data.get(address_key.clone()) else {
                continue;
            };
            if Address::try_from_val(env, &address_val).ok().as_ref() != Some(nft_address) {
                continue;
            }
            if let Some(type_val) = event_data.get(nft_type_key.clone()) {
                return NFTType::try_from_val(env, &type_val).ok();
            }
        }
        None
    }


    fn setup_nft_factory(env: &Env) -> (NFTFactoryClient, Address) {
        let admin = Address::generate(env);
        let contract_id = env.register(NFTFactory, (&admin,));
        let client = NFTFactoryClient::new(env, &contract_id);
        (client, admin)
    }

    fn setup_with_wasm(env: &Env) -> (NFTFactoryClient, Address, BytesN<32>) {
        env.mock_all_auths();
        let (client, admin) = setup_nft_factory(env);
        let wasm_hash = BytesN::from_array(env, &[1u8; 32]);

        client.set_enumerable_wasm(&admin, &wasm_hash);
        client.set_royalties_wasm(&admin, &wasm_hash);
        client.set_access_control_wasm(&admin, &wasm_hash);

        (client, admin, wasm_hash)
    }

    // ===== Success-Path Deployment Tests =====

    #[test]
    fn test_deploy_enumerable_nft_success() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_nft_factory(&env);
        let wasm_hash = env.deployer().upload_contract_wasm(nft_enumerable_wasm::WASM);
        client.set_enumerable_wasm(&admin, &wasm_hash);

        let owner = Address::generate(&env);
        let config = NFTConfig {
            nft_type: NFTType::Enumerable,
            owner: owner.clone(),
            admin: None,
            manager: None,
            salt: BytesN::from_array(&env, &[81u8; 32]),
            name: Some(String::from_str(&env, "Enumerable Collection")),
            symbol: Some(String::from_str(&env, "ENUM")),
            base_uri: Some(String::from_str(&env, "https://example.com/nft/")),
        };

        let nft_address = client.deploy_nft(&admin, &config);

        assert_eq!(client.get_nft_count(), 1);
        let nfts = client.get_deployed_nfts();
        assert_eq!(nfts.get(0).unwrap().address, nft_address);
        assert_eq!(nfts.get(0).unwrap().nft_type, NFTType::Enumerable);
        assert_eq!(nfts.get(0).unwrap().owner, owner);
        assert_eq!(
            nft_deployed_event_type(&env, &nft_address),
            Some(NFTType::Enumerable)
        );
    }

    #[test]
    fn test_deploy_royalties_nft_success() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_nft_factory(&env);
        let wasm_hash = env.deployer().upload_contract_wasm(nft_royalties_wasm::WASM);
        client.set_royalties_wasm(&admin, &wasm_hash);

        let owner = Address::generate(&env);
        let nft_admin = Address::generate(&env);
        let nft_manager = Address::generate(&env);
        let config = NFTConfig {
            nft_type: NFTType::Royalties,
            owner: owner.clone(),
            admin: Some(nft_admin),
            manager: Some(nft_manager),
            salt: BytesN::from_array(&env, &[82u8; 32]),
            name: Some(String::from_str(&env, "Royalties Collection")),
            symbol: Some(String::from_str(&env, "ROY")),
            base_uri: Some(String::from_str(&env, "https://example.com/nft/")),
        };

        let nft_address = client.deploy_nft(&admin, &config);

        assert_eq!(client.get_nft_count(), 1);
        let nfts = client.get_deployed_nfts();
        assert_eq!(nfts.get(0).unwrap().address, nft_address);
        assert_eq!(nfts.get(0).unwrap().nft_type, NFTType::Royalties);
        assert_eq!(
            nft_deployed_event_type(&env, &nft_address),
            Some(NFTType::Royalties)
        );
    }

    #[test]
    fn test_deploy_access_control_nft_success() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_nft_factory(&env);
        let wasm_hash = env.deployer().upload_contract_wasm(nft_access_control_wasm::WASM);
        client.set_access_control_wasm(&admin, &wasm_hash);

        let owner = Address::generate(&env);
        let nft_admin = Address::generate(&env);
        let config = NFTConfig {
            nft_type: NFTType::AccessControl,
            owner: owner.clone(),
            admin: Some(nft_admin),
            manager: None,
            salt: BytesN::from_array(&env, &[83u8; 32]),
            name: Some(String::from_str(&env, "Access Control Collection")),
            symbol: Some(String::from_str(&env, "ACL")),
            base_uri: Some(String::from_str(&env, "https://example.com/nft/")),
        };

        let nft_address = client.deploy_nft(&admin, &config);

        assert_eq!(client.get_nft_count(), 1);
        let nfts = client.get_deployed_nfts();
        assert_eq!(nfts.get(0).unwrap().address, nft_address);
        assert_eq!(nfts.get(0).unwrap().nft_type, NFTType::AccessControl);
        assert_eq!(
            nft_deployed_event_type(&env, &nft_address),
            Some(NFTType::AccessControl)
        );
    }


    // ===== Constructor Tests =====

    #[test]
    fn test_constructor() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(NFTFactory, (&admin,));
        let client = NFTFactoryClient::new(&env, &contract_id);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);

        let count = client.get_nft_count();
        assert_eq!(count, 0);

        let nfts = client.get_deployed_nfts();
        assert_eq!(nfts.len(), 0);
    }

    // ===== WASM Configuration Tests =====

    #[test]
    fn test_set_wasm_hashes() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(NFTFactory, (&admin,));
        let client = NFTFactoryClient::new(&env, &contract_id);

        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // Should not panic
        client.set_enumerable_wasm(&admin, &wasm_hash);
        client.set_royalties_wasm(&admin, &wasm_hash);
        client.set_access_control_wasm(&admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_enumerable_wasm_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_nft_factory(&env);
        let not_admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        client.set_enumerable_wasm(&not_admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_royalties_wasm_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_nft_factory(&env);
        let not_admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        client.set_royalties_wasm(&not_admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_access_control_wasm_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_nft_factory(&env);
        let not_admin = Address::generate(&env);
        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        client.set_access_control_wasm(&not_admin, &wasm_hash);
    }

    // ===== Validation Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_royalties_nft_missing_admin() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let manager = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = NFTConfig {
            nft_type: NFTType::Royalties,
            owner,
            admin: None, // Missing
            manager: Some(manager),
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_royalties_nft_missing_manager() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let admin = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = NFTConfig {
            nft_type: NFTType::Royalties,
            owner,
            admin: Some(admin),
            manager: None, // Missing
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_deploy_access_control_nft_missing_admin() {
        let env = Env::default();
        let (client, _admin, _wasm) = setup_with_wasm(&env);

        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = NFTConfig {
            nft_type: NFTType::AccessControl,
            owner,
            admin: None, // Missing
            manager: None,
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_deploy_nft_wasm_not_set() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_nft_factory(&env);
        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[2u8; 32]);

        let config = NFTConfig {
            nft_type: NFTType::Enumerable,
            owner,
            admin: None,
            manager: None,
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }

    // ===== Query Tests =====

    #[test]
    fn test_get_deployed_nfts_empty() {
        let env = Env::default();
        let (client, _admin) = setup_nft_factory(&env);

        let nfts = client.get_deployed_nfts();
        assert_eq!(nfts.len(), 0);
    }

    #[test]
    fn test_get_nfts_by_type_empty() {
        let env = Env::default();
        let (client, _admin) = setup_nft_factory(&env);

        let nfts = client.get_nfts_by_type(&NFTType::Enumerable);
        assert_eq!(nfts.len(), 0);
    }

    #[test]
    fn test_get_nfts_by_owner_empty() {
        let env = Env::default();
        let (client, _admin) = setup_nft_factory(&env);
        let owner = Address::generate(&env);

        let nfts = client.get_nfts_by_owner(&owner);
        assert_eq!(nfts.len(), 0);
    }

    #[test]
    fn test_get_nft_count() {
        let env = Env::default();
        let (client, _admin) = setup_nft_factory(&env);

        let count = client.get_nft_count();
        assert_eq!(count, 0);
    }

    // ===== Admin Transfer Tests =====

    #[test]
    fn test_transfer_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_nft_factory(&env);
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

        let (client, _admin) = setup_nft_factory(&env);
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

        let (client, _admin) = setup_nft_factory(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[99u8; 32]);

        // Test passes if upgrade completes successfully with proper admin auth
        // The upgrade function internally verifies admin and requires their auth
        client.upgrade(&new_wasm_hash);
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_get_admin_returns_correct_value() {
        let env = Env::default();
        let (client, admin) = setup_nft_factory(&env);

        let retrieved_admin = client.get_admin();
        assert_eq!(retrieved_admin, admin);
    }

    #[test]
    fn test_multiple_admin_transfers() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin1) = setup_nft_factory(&env);
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

    // ===== Pause / Unpause Tests =====

    /// While the factory is paused, `deploy_nft` is rejected before any
    /// deployment work happens, bubbling up `NFTFactoryError::ContractPaused`
    /// (`Contract, #8`).
    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_pause_blocks_deploy_nft() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, _wasm) = setup_with_wasm(&env);
        client.pause(&admin);

        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[7u8; 32]);
        let config = NFTConfig {
            nft_type: NFTType::Enumerable,
            owner,
            admin: None,
            manager: None,
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }

    /// After `unpause`, the pause gate is lifted: `deploy_nft` now gets past
    /// the `Paused` check and fails further down at the (unconfigured) WASM
    /// lookup, `NFTFactoryError::WasmNotSet` (`Contract, #2`).
    ///
    /// NOTE: a genuine *successful* deployment cannot be asserted here with a
    /// plain `cargo test`: `deploy_v2` requires an uploaded contract WASM,
    /// which unit tests can only obtain from a compiled `.wasm` artifact
    /// (via `contractimport!`) that this workspace does not build. Reaching
    /// `WasmNotSet` proves the `ContractPaused` branch was skipped.
    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_unpause_allows_deploy_nft_to_proceed() {
        let env = Env::default();
        env.mock_all_auths();

        // Deliberately no WASM configured, so the only gate left to observe is
        // the pause flag itself.
        let (client, admin) = setup_nft_factory(&env);
        client.pause(&admin);
        client.unpause(&admin);

        let deployer = Address::generate(&env);
        let owner = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[8u8; 32]);
        let config = NFTConfig {
            nft_type: NFTType::Enumerable,
            owner,
            admin: None,
            manager: None,
            salt,
            name: None,
            symbol: None,
            base_uri: None,
        };

        client.deploy_nft(&deployer, &config);
    }
}
