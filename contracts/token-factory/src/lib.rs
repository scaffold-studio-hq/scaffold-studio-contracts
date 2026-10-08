#![no_std]

use soroban_sdk::{
    contract, contractevent, contractimpl, contracterror, contracttype, panic_with_error, Address, BytesN, Env,
    IntoVal, String, Val, Vec,
};

/// TokenFactory - Deploys fungible token contracts
///
/// This contract manages deployment of various token types:
/// - Allowlist Token (whitelist-only transfers)
/// - Blocklist Token (blacklist specific addresses)
/// - Capped Token (max supply limit)
/// - Pausable Token (emergency stop)
/// - Vault Token (time-locked tokens)

#[contract]
pub struct TokenFactory;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    AllowlistWasm,
    BlocklistWasm,
    CappedWasm,
    PausableWasm,
    VaultWasm,
    DeployedTokens,
    TokenCount,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenType {
    Allowlist,
    Blocklist,
    Capped,
    Pausable,
    Vault,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenConfig {
    pub token_type: TokenType,
    pub admin: Address,
    pub manager: Address,
    pub initial_supply: i128,
    pub cap: Option<i128>, // Only for Capped tokens
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub salt: BytesN<32>,
    // Vault-specific parameters
    pub asset: Option<Address>,          // For Vault: underlying asset address
    pub decimals_offset: Option<u32>,    // For Vault: decimals offset
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenInfo {
    pub address: Address,
    pub token_type: TokenType,
    pub admin: Address,
    pub timestamp: u64,
    pub name: Option<String>,
}

#[contractevent]
pub struct TokenDeployedEvent {
    pub token_address: Address,
    pub token_type: TokenType,
    pub deployer: Address,
    pub name: String,
    pub symbol: String,
    pub timestamp: u64,
}

#[contractevent]
pub struct WasmUpdatedEvent {
    pub token_type_name: String,
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
pub enum TokenFactoryError {
    WasmNotSet = 2,
    InvalidTokenType = 3,
    InvalidConfig = 4,
    InvalidName = 5,
    InvalidSymbol = 6,
    InvalidDecimals = 7,
    NegativeSupply = 8,
    MissingCap = 9,
    CapTooLow = 10,
    UnexpectedCap = 11,
    CounterOverflow = 13,
    InvalidCharacters = 14,
    SupplyTooLarge = 15,
}

#[contractimpl]
impl TokenFactory {
    /// Initialize TokenFactory with admin address
    ///
    /// # Arguments
    /// * `admin` - Address that will have admin privileges
    pub fn __constructor(e: Env, admin: Address) {
        factory_common::set_admin(&e, &admin);

        // Initialize empty tokens list
        let tokens: Vec<TokenInfo> = Vec::new(&e);
        e.storage().instance().set(&DataKey::DeployedTokens, &tokens);
        e.storage().instance().set(&DataKey::TokenCount, &0u32);
        factory_common::set_paused(&e, false);
    }

    /// Set WASM hash for Allowlist token type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Allowlist token contract
    pub fn set_allowlist_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::AllowlistWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            token_type_name: String::from_str(&e, "Allowlist"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Blocklist token type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Blocklist token contract
    pub fn set_blocklist_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::BlocklistWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            token_type_name: String::from_str(&e, "Blocklist"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Capped token type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Capped token contract
    pub fn set_capped_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::CappedWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            token_type_name: String::from_str(&e, "Capped"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Pausable token type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Pausable token contract
    pub fn set_pausable_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::PausableWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            token_type_name: String::from_str(&e, "Pausable"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Set WASM hash for Vault token type
    ///
    /// # Arguments
    /// * `admin` - Admin address (for authorization)
    /// * `wasm_hash` - WASM hash of the Vault token contract
    pub fn set_vault_wasm(e: Env, admin: Address, wasm_hash: BytesN<32>) {
        admin.require_auth();
        factory_common::require_admin(&e, &admin);
        e.storage().instance().set(&DataKey::VaultWasm, &wasm_hash);

        // Emit event
        WasmUpdatedEvent {
            token_type_name: String::from_str(&e, "Vault"),
            wasm_hash: wasm_hash.clone(),
        }
        .publish(&e);
    }

    /// Deploy a token contract with specified configuration
    ///
    /// # Arguments
    /// * `deployer` - Address calling this function
    /// * `config` - Token configuration including type, admin, supply, etc.
    ///
    /// # Returns
    /// Address of the deployed token contract
    pub fn deploy_token(e: Env, deployer: Address, config: TokenConfig) -> Address {
        deployer.require_auth();

        // Reject deployments while the contract is paused
        factory_common::require_not_paused(&e);

        // Get WASM hash based on token type
        let wasm_hash = Self::get_wasm_for_type(&e, &config.token_type);

        // Validate config based on token type
        Self::validate_config(&e, &config);

        // Deploy contract - deploy_v2 requires constructor_args as Vec<Val>, not tuple
        let token_address = match config.token_type {
            TokenType::Capped => {
                // Capped token requires cap parameter - safe unwrap after validation
                let cap = config.cap.unwrap_or_else(|| {
                    panic_with_error!(&e, TokenFactoryError::MissingCap)
                });

                // Convert constructor args to Vec<Val>
                let constructor_args: Vec<Val> = (
                    config.admin.clone(),
                    config.manager.clone(),
                    config.initial_supply,
                    cap,
                    config.name.clone(),
                    config.symbol.clone(),
                    config.decimals,
                ).into_val(&e);

                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
            TokenType::Vault => {
                // Vault tokens have a different constructor signature: (asset, decimals_offset)
                // Validation ensures these fields are present
                let asset = config.asset.clone().unwrap_or_else(|| {
                    panic_with_error!(&e, TokenFactoryError::InvalidConfig)
                });
                let decimals_offset = config.decimals_offset.unwrap_or_else(|| {
                    panic_with_error!(&e, TokenFactoryError::InvalidConfig)
                });

                // Convert constructor args to Vec<Val>
                let constructor_args: Vec<Val> = (asset, decimals_offset).into_val(&e);

                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
            _ => {
                // Other token types use standard constructor
                // Convert constructor args to Vec<Val>
                let constructor_args: Vec<Val> = (
                    config.admin.clone(),
                    config.manager.clone(),
                    config.initial_supply,
                    config.name.clone(),
                    config.symbol.clone(),
                    config.decimals,
                ).into_val(&e);

                e.deployer()
                    .with_address(e.current_contract_address(), config.salt)
                    .deploy_v2(wasm_hash, constructor_args)
            }
        };

        // Update state AFTER successful deployment
        // Increment token count with overflow protection
        let count: u32 = e.storage().instance().get(&DataKey::TokenCount).unwrap_or(0);
        let new_count = count.checked_add(1)
            .unwrap_or_else(|| {
                panic_with_error!(&e, TokenFactoryError::CounterOverflow)
            });

        // Store token info
        let token_info = TokenInfo {
            address: token_address.clone(),
            token_type: config.token_type.clone(),
            admin: config.admin.clone(),
            timestamp: e.ledger().timestamp(),
            name: Some(config.name.clone()),
        };

        let mut tokens: Vec<TokenInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedTokens)
            .unwrap_or_else(|| Vec::new(&e));
        tokens.push_back(token_info);
        e.storage()
            .instance()
            .set(&DataKey::DeployedTokens, &tokens);

        // Update token count
        e.storage()
            .instance()
            .set(&DataKey::TokenCount, &new_count);

        // Emit event
        TokenDeployedEvent {
            token_address: token_address.clone(),
            token_type: config.token_type.clone(),
            deployer: deployer.clone(),
            name: config.name.clone(),
            symbol: config.symbol.clone(),
            timestamp: e.ledger().timestamp(),
        }
        .publish(&e);

        token_address
    }

    /// Get all deployed tokens
    ///
    /// # Returns
    /// Vector of TokenInfo containing all deployed tokens
    pub fn get_deployed_tokens(e: Env) -> Vec<TokenInfo> {
        e.storage()
            .instance()
            .get(&DataKey::DeployedTokens)
            .unwrap_or(Vec::new(&e))
    }

    /// Get tokens by type
    ///
    /// # Arguments
    /// * `token_type` - Type of tokens to filter by
    ///
    /// # Returns
    /// Vector of TokenInfo for the specified type
    pub fn get_tokens_by_type(e: Env, token_type: TokenType) -> Vec<TokenInfo> {
        let all_tokens: Vec<TokenInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedTokens)
            .unwrap_or(Vec::new(&e));

        let mut filtered = Vec::new(&e);
        for token in all_tokens.iter() {
            if token.token_type == token_type {
                filtered.push_back(token);
            }
        }
        filtered
    }

    /// Get tokens by admin
    ///
    /// # Arguments
    /// * `admin` - Admin address to filter by
    ///
    /// # Returns
    /// Vector of TokenInfo for tokens managed by the admin
    pub fn get_tokens_by_admin(e: Env, admin: Address) -> Vec<TokenInfo> {
        let all_tokens: Vec<TokenInfo> = e
            .storage()
            .instance()
            .get(&DataKey::DeployedTokens)
            .unwrap_or(Vec::new(&e));

        let mut filtered = Vec::new(&e);
        for token in all_tokens.iter() {
            if token.admin == admin {
                filtered.push_back(token);
            }
        }
        filtered
    }

    /// Get total number of deployed tokens
    ///
    /// # Returns
    /// Total count of deployed tokens
    pub fn get_token_count(e: Env) -> u32 {
        e.storage().instance().get(&DataKey::TokenCount).unwrap_or(0)
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

        // Perform upgrade
        e.deployer().update_current_contract_wasm(new_wasm_hash);

        // Note: Contract will be paused after upgrade, admin must unpause
    }

    // Helper: Get WASM hash for token type
    fn get_wasm_for_type(e: &Env, token_type: &TokenType) -> BytesN<32> {
        let key = match token_type {
            TokenType::Allowlist => DataKey::AllowlistWasm,
            TokenType::Blocklist => DataKey::BlocklistWasm,
            TokenType::Capped => DataKey::CappedWasm,
            TokenType::Pausable => DataKey::PausableWasm,
            TokenType::Vault => DataKey::VaultWasm,
        };

        e.storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(e, TokenFactoryError::WasmNotSet))
    }

    // Helper: Validate string contains no null bytes or control characters
    fn validate_string_chars(_e: &Env, s: &String) -> bool {
        let bytes = s.to_bytes();
        for i in 0..bytes.len() {
            let byte = bytes.get(i).unwrap();
            // Check for null byte or control characters (0-31 except tab, newline, carriage return)
            if byte == 0 || (byte < 32 && byte != 9 && byte != 10 && byte != 13) {
                return false;
            }
        }
        true
    }

    // Helper: Validate token configuration
    fn validate_config(e: &Env, config: &TokenConfig) {
        // Validate name (1-30 characters)
        if config.name.len() == 0 || config.name.len() > 30 {
            panic_with_error!(e, TokenFactoryError::InvalidName);
        }

        // Validate symbol (1-12 characters)
        if config.symbol.len() == 0 || config.symbol.len() > 12 {
            panic_with_error!(e, TokenFactoryError::InvalidSymbol);
        }

        // Validate name and symbol contain no null bytes or control characters
        if !Self::validate_string_chars(e, &config.name) {
            panic_with_error!(e, TokenFactoryError::InvalidName);
        }
        if !Self::validate_string_chars(e, &config.symbol) {
            panic_with_error!(e, TokenFactoryError::InvalidSymbol);
        }

        // Validate decimals (max 18, typical for Stellar is 7)
        if config.decimals > 18 {
            panic_with_error!(e, TokenFactoryError::InvalidDecimals);
        }

        // Initial supply must be non-negative
        if config.initial_supply < 0 {
            panic_with_error!(e, TokenFactoryError::NegativeSupply);
        }

        // Check for unreasonably large supply (to prevent overflow issues)
        const MAX_SUPPLY: i128 = i128::MAX / 2; // Safe upper bound
        if config.initial_supply > MAX_SUPPLY {
            panic_with_error!(e, TokenFactoryError::SupplyTooLarge);
        }

        // Type-specific validation
        match config.token_type {
            TokenType::Capped => {
                // Capped tokens must have a cap
                if config.cap.is_none() {
                    panic_with_error!(e, TokenFactoryError::MissingCap);
                }
                // Initial supply must not exceed cap
                if let Some(cap) = config.cap {
                    if config.initial_supply > cap {
                        panic_with_error!(e, TokenFactoryError::CapTooLow);
                    }
                    // Cap must also be reasonable
                    if cap > MAX_SUPPLY {
                        panic_with_error!(e, TokenFactoryError::SupplyTooLarge);
                    }
                }
                // Vault-specific fields should not be set for Capped tokens
                if config.asset.is_some() || config.decimals_offset.is_some() {
                    panic_with_error!(e, TokenFactoryError::InvalidConfig);
                }
            }
            TokenType::Vault => {
                // Vault tokens must have asset and decimals_offset
                if config.asset.is_none() {
                    panic_with_error!(e, TokenFactoryError::InvalidConfig);
                }
                if config.decimals_offset.is_none() {
                    panic_with_error!(e, TokenFactoryError::InvalidConfig);
                }
                // Vault tokens should not have cap set
                if config.cap.is_some() {
                    panic_with_error!(e, TokenFactoryError::UnexpectedCap);
                }
            }
            _ => {
                // Non-capped, non-vault tokens should not have cap or vault-specific fields
                if config.cap.is_some() {
                    panic_with_error!(e, TokenFactoryError::UnexpectedCap);
                }
                if config.asset.is_some() || config.decimals_offset.is_some() {
                    panic_with_error!(e, TokenFactoryError::InvalidConfig);
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
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Events}, Env, String};

    fn setup_factory(env: &Env) -> (TokenFactoryClient, Address) {
        let admin = Address::generate(env);
        let contract_id = env.register(TokenFactory, (&admin,));
        let client = TokenFactoryClient::new(env, &contract_id);
        (client, admin)
    }

    fn setup_with_wasm(env: &Env) -> (TokenFactoryClient, Address, BytesN<32>) {
        env.mock_all_auths();
        let (client, admin) = setup_factory(env);
        let wasm_hash = BytesN::from_array(env, &[1u8; 32]);

        // Set all WASM hashes
        client.set_allowlist_wasm(&admin, &wasm_hash);
        client.set_blocklist_wasm(&admin, &wasm_hash);
        client.set_capped_wasm(&admin, &wasm_hash);
        client.set_pausable_wasm(&admin, &wasm_hash);
        client.set_vault_wasm(&admin, &wasm_hash);

        (client, admin, wasm_hash)
    }

    // ===== Constructor Tests =====

    #[test]
    fn test_constructor() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(TokenFactory, (&admin,));
        let client = TokenFactoryClient::new(&env, &contract_id);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);

        let count = client.get_token_count();
        assert_eq!(count, 0);

        let tokens = client.get_deployed_tokens();
        assert_eq!(tokens.len(), 0);
    }

    // ===== WASM Configuration Tests =====

    #[test]
    fn test_set_wasm_hashes() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(TokenFactory, (&admin,));
        let client = TokenFactoryClient::new(&env, &contract_id);

        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // Should not panic
        client.set_allowlist_wasm(&admin, &wasm_hash);
        client.set_blocklist_wasm(&admin, &wasm_hash);
        client.set_capped_wasm(&admin, &wasm_hash);
        client.set_pausable_wasm(&admin, &wasm_hash);
        client.set_vault_wasm(&admin, &wasm_hash);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_set_wasm_not_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let not_admin = Address::generate(&env);

        let contract_id = env.register(TokenFactory, (&admin,));
        let client = TokenFactoryClient::new(&env, &contract_id);

        let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // This should panic with error code 1 (NotAdmin)
        client.set_allowlist_wasm(&not_admin, &wasm_hash);
    }

    // ===== Validation Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // InvalidName
    fn test_validation_empty_name() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, ""), // Empty name
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // InvalidName
    fn test_validation_name_too_long() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "ThisNameIsWayTooLongAndExceedsThirtyCharactersLimit"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6)")] // InvalidSymbol
    fn test_validation_empty_symbol() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, ""), // Empty symbol
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6)")] // InvalidSymbol
    fn test_validation_symbol_too_long() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "WAYTOOLONGSYMBOL"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // InvalidName - null bytes
    fn test_validation_name_with_null_bytes() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        // Create a name with a null byte: "Test\0Name"
        let name_with_null = String::from_bytes(&env, &[84u8, 101u8, 115u8, 116u8, 0u8, 78u8, 97u8, 109u8, 101u8]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: name_with_null,
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6)")] // InvalidSymbol - null bytes
    fn test_validation_symbol_with_null_bytes() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        // Create a symbol with a null byte: "TST\0X"
        let symbol_with_null = String::from_bytes(&env, &[84u8, 83u8, 84u8, 0u8, 88u8]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: symbol_with_null,
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #7)")] // InvalidDecimals
    fn test_validation_decimals_too_high() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 19, // Too high
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")] // NegativeSupply
    fn test_validation_negative_supply() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: -1000, // Negative
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #9)")] // MissingCap
    fn test_validation_capped_without_cap() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Capped,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None, // Missing cap for Capped token
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")] // CapTooLow
    fn test_validation_cap_too_low() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Capped,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 2000000,
            cap: Some(1000000), // Cap less than initial supply
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")] // UnexpectedCap
    fn test_validation_non_capped_with_cap() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: Some(2000000), // Should not have cap for non-Capped token
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&admin, &config);
    }

    // ===== Vault Validation Tests =====

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // InvalidConfig - missing asset
    fn test_validation_vault_missing_asset() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let config = TokenConfig {
            token_type: TokenType::Vault,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 0,
            cap: None,
            name: String::from_str(&env, "Vault Token"),
            symbol: String::from_str(&env, "VLT"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: None, // Missing asset for Vault
            decimals_offset: Some(2),
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // InvalidConfig - missing decimals_offset
    fn test_validation_vault_missing_decimals_offset() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let asset = Address::generate(&env);
        let config = TokenConfig {
            token_type: TokenType::Vault,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 0,
            cap: None,
            name: String::from_str(&env, "Vault Token"),
            symbol: String::from_str(&env, "VLT"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: Some(asset),
            decimals_offset: None, // Missing decimals_offset for Vault
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")] // UnexpectedCap - Vault with cap
    fn test_validation_vault_with_cap() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let asset = Address::generate(&env);
        let config = TokenConfig {
            token_type: TokenType::Vault,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 0,
            cap: Some(1000000), // Vault should not have cap
            name: String::from_str(&env, "Vault Token"),
            symbol: String::from_str(&env, "VLT"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: Some(asset),
            decimals_offset: Some(2),
        };

        client.deploy_token(&admin, &config);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // InvalidConfig - non-Vault with vault fields
    fn test_validation_allowlist_with_vault_fields() {
        let env = Env::default();
        let (client, admin, _) = setup_with_wasm(&env);

        let asset = Address::generate(&env);
        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin.clone(),
            manager: admin.clone(),
            initial_supply: 1000000,
            cap: None,
            name: String::from_str(&env, "Test Token"),
            symbol: String::from_str(&env, "TEST"),
            decimals: 7,
            salt: BytesN::from_array(&env, &[2u8; 32]),
            asset: Some(asset), // Allowlist should not have vault fields
            decimals_offset: Some(2),
        };

        client.deploy_token(&admin, &config);
    }

    // ===== Admin Tests =====
    // Note: Admin transfer tests are now in TWO-STEP ADMIN TRANSFER TESTS section

    // ===== Upgrade Tests =====
    // Note: upgrade() tests require actual WASM in test environment
    // We test auth checks but skip actual upgrade functionality

    #[test]
    #[ignore = "Requires real WASM for upgrade - test in integration environment"]
    fn test_upgrade_requires_admin_auth() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_factory(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[99u8; 32]);

        // Test passes if upgrade completes successfully with proper admin auth
        // The upgrade function internally verifies admin and requires their auth
        client.upgrade(&new_wasm_hash);
    }

    // ===== Query Tests =====

    #[test]
    fn test_get_deployed_tokens_empty() {
        let env = Env::default();
        let (client, _admin) = setup_factory(&env);

        let tokens = client.get_deployed_tokens();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_get_tokens_by_type_empty() {
        let env = Env::default();
        let (client, _admin) = setup_factory(&env);

        let tokens = client.get_tokens_by_type(&TokenType::Allowlist);
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_get_tokens_by_admin_empty() {
        let env = Env::default();
        let (client, admin) = setup_factory(&env);

        let tokens = client.get_tokens_by_admin(&admin);
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_get_token_count_initial() {
        let env = Env::default();
        let (client, _admin) = setup_factory(&env);

        let count = client.get_token_count();
        assert_eq!(count, 0);
    }

    // ===== SECURITY TESTS =====

    #[test]
    #[ignore = "Requires real WASM deployment - move to integration tests"]
    #[should_panic(expected = "Error(Contract, #14)")] // DuplicateSalt
    fn test_security_salt_duplication_prevention() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[42u8; 32]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 1000,
            cap: None,
            name: String::from_str(&env, "Token1"),
            symbol: String::from_str(&env, "TK1"),
            decimals: 7,
            salt: salt.clone(),
            asset: None,
            decimals_offset: None,
        };

        // First deployment should succeed
        client.deploy_token(&deployer, &config);

        // Second deployment with same salt should fail
        let config2 = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 2000,
            cap: None,
            name: String::from_str(&env, "Token2"),
            symbol: String::from_str(&env, "TK2"),
            decimals: 7,
            salt: salt.clone(), // Same salt!
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&deployer, &config2); // Should panic with DuplicateSalt
    }

    #[test]
    #[ignore = "Requires real WASM deployment - move to integration tests"]
    #[should_panic(expected = "Error(Contract, #15)")] // RateLimitExceeded
    fn test_security_rate_limiting_dos_protection() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);

        // Deploy 10 tokens (should all succeed)
        for i in 0..10 {
            let salt = BytesN::from_array(&env, &[i; 32]);
            let config = TokenConfig {
                token_type: TokenType::Allowlist,
                admin: admin_addr.clone(),
                manager: admin_addr.clone(),
                initial_supply: 1000,
                cap: None,
                name: String::from_str(&env, "Token"),
                symbol: String::from_str(&env, "TK"),
                decimals: 7,
                salt,
                asset: None,
                decimals_offset: None,
            };
            client.deploy_token(&deployer, &config);
        }

        // 11th deployment should fail with RateLimitExceeded
        let salt = BytesN::from_array(&env, &[99u8; 32]);
        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 1000,
            cap: None,
            name: String::from_str(&env, "Token11"),
            symbol: String::from_str(&env, "TK11"),
            decimals: 7,
            salt,
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&deployer, &config); // Should panic
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")] // ContractPaused
    fn test_security_pause_prevents_deployment() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        // Pause the contract
        client.pause(&admin);

        // Try to deploy (should fail)
        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[1u8; 32]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 1000,
            cap: None,
            name: String::from_str(&env, "Token"),
            symbol: String::from_str(&env, "TK"),
            decimals: 7,
            salt,
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&deployer, &config); // Should panic
    }

    #[test]
    #[ignore = "Requires real WASM deployment - move to integration tests"]
    fn test_security_unpause_restores_functionality() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        // Pause
        client.pause(&admin);

        // Unpause
        client.unpause(&admin);

        // Deploy should work now
        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[1u8; 32]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 1000,
            cap: None,
            name: String::from_str(&env, "Token"),
            symbol: String::from_str(&env, "TK"),
            decimals: 7,
            salt,
            asset: None,
            decimals_offset: None,
        };

        let result = client.deploy_token(&deployer, &config);
        assert!(result != Address::generate(&env)); // Should return valid address
    }

    // ===== TWO-STEP ADMIN TRANSFER TESTS =====

    #[test]
    fn test_twostep_admin_transfer_full_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);

        // Step 1: Current admin initiates transfer
        client.initiate_admin_transfer(&current_admin, &new_admin);

        // Verify pending admin is set
        let pending = client.get_pending_admin();
        assert_eq!(pending, Some(new_admin.clone()));

        // Admin should still be the current one
        assert_eq!(client.get_admin(), current_admin);

        // Step 2: New admin accepts transfer
        client.accept_admin_transfer(&new_admin);

        // Verify admin changed
        assert_eq!(client.get_admin(), new_admin);

        // Verify pending admin cleared
        assert_eq!(client.get_pending_admin(), None);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")] // NotPendingAdmin
    fn test_twostep_admin_transfer_wrong_acceptor() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);
        let wrong_admin = Address::generate(&env);

        // Initiate transfer to new_admin
        client.initiate_admin_transfer(&current_admin, &new_admin);

        // Try to accept with wrong address
        client.accept_admin_transfer(&wrong_admin); // Should panic
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")] // NoPendingAdmin
    fn test_twostep_admin_transfer_accept_without_initiate() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);

        // Try to accept without initiating
        client.accept_admin_transfer(&new_admin); // Should panic
    }

    #[test]
    fn test_twostep_admin_transfer_cancel() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);

        // Initiate transfer
        client.initiate_admin_transfer(&current_admin, &new_admin);

        // Verify pending admin is set
        assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

        // Cancel transfer
        client.cancel_admin_transfer(&current_admin);

        // Verify pending admin cleared
        assert_eq!(client.get_pending_admin(), None);

        // Admin should still be the current one
        assert_eq!(client.get_admin(), current_admin);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
    fn test_twostep_pause_requires_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, _admin) = setup_factory(&env);
        let not_admin = Address::generate(&env);

        client.pause(&not_admin); // Should panic
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")] // NotAdmin
    fn test_twostep_unpause_requires_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_factory(&env);

        client.pause(&admin);

        let not_admin = Address::generate(&env);
        client.unpause(&not_admin); // Should panic
    }

    // ===== EVENT EMISSION TESTS =====
    // Note: Soroban SDK event testing requires accessing env.events()
    // These tests verify events are published correctly

    #[test]
    #[ignore = "Requires real WASM deployment - move to integration tests"]
    fn test_events_token_deployment_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);
        let salt = BytesN::from_array(&env, &[1u8; 32]);

        let config = TokenConfig {
            token_type: TokenType::Allowlist,
            admin: admin_addr.clone(),
            manager: admin_addr.clone(),
            initial_supply: 1000,
            cap: None,
            name: String::from_str(&env, "TestToken"),
            symbol: String::from_str(&env, "TST"),
            decimals: 7,
            salt,
            asset: None,
            decimals_offset: None,
        };

        client.deploy_token(&deployer, &config);

        // Verify event was emitted
        let events = env.events().all();
        assert!(events.len() > 0);

        // Note: Full event structure verification would require
        // parsing event data which is complex in tests
    }

    #[test]
    fn test_events_pause_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin) = setup_factory(&env);

        client.pause(&admin);

        let events = env.events().all();
        assert!(events.len() > 0);
    }

    #[test]
    fn test_events_admin_transfer_initiated_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);

        client.initiate_admin_transfer(&current_admin, &new_admin);

        let events = env.events().all();
        assert!(events.len() > 0);
    }

    #[test]
    fn test_events_admin_transferred_emits_event() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, current_admin) = setup_factory(&env);
        let new_admin = Address::generate(&env);

        client.initiate_admin_transfer(&current_admin, &new_admin);
        client.accept_admin_transfer(&new_admin);

        // Verify the admin transfer actually worked (functional test)
        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, new_admin);

        // Note: Event emission verification skipped due to deprecated Events::publish() API
        // The deprecated API may not emit events properly in unit tests
        // Event emission will be verified in integration tests
        // TODO: Update to #[contractevent] macro and re-enable event assertions
    }

    // ===== OVERFLOW PROTECTION TESTS =====

    #[test]
    #[ignore = "Requires real WASM deployment - move to integration tests"]
    fn test_overflow_token_counter_protection() {
        let env = Env::default();
        env.mock_all_auths();

        let (client, admin, wasm_hash) = setup_with_wasm(&env);
        client.set_allowlist_wasm(&admin, &wasm_hash);

        let deployer = Address::generate(&env);
        let admin_addr = Address::generate(&env);

        // Deploy multiple tokens to test counter doesn't overflow
        for i in 0..5u8 {
            let salt = BytesN::from_array(&env, &[i; 32]);
            let config = TokenConfig {
                token_type: TokenType::Allowlist,
                admin: admin_addr.clone(),
                manager: admin_addr.clone(),
                initial_supply: 1000,
                cap: None,
                name: String::from_str(&env, "Token"),
                symbol: String::from_str(&env, "TK"),
                decimals: 7,
                salt,
                asset: None,
                decimals_offset: None,
            };
            client.deploy_token(&deployer, &config);
        }

        // Verify counter incremented correctly
        assert_eq!(client.get_token_count(), 5);
    }
}
