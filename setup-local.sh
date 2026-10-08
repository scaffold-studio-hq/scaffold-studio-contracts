#!/bin/bash
# Local Network Factory Initialization Script
# This script:
# 1. Builds all contracts
# 2. Uploads WASMs to local standalone network
# 3. Initializes all factories with WASM hashes

set -e

echo "======================================"
echo "Stellar Studio Local Setup"
echo "======================================"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
NETWORK="local"  # Use "local" instead of "standalone" for stellar CLI
ADMIN_IDENTITY="me" # Uses stellar CLI identity

# Factory addresses on the current local network.
# NOTE: local addresses are ephemeral - they change whenever the Stellar Docker
# container is recreated. Re-run this script after a reset and read the addresses
# it prints at the end.
MASTER_FACTORY="CCNVPWKVKVLNPYWXBEMH4QEQ6HMVSOJBQ3GGHW55L6IJY4EXGQCXYTBU"
TOKEN_FACTORY="CCAX3IFONMYLL3RJA3MFUGG4NPRL2AE376TJFTBG6XUL64H6QMR3UJGM"
NFT_FACTORY="CAJ75GRCGSRZXNFQFFIQNLS22STHMQESOT4LDAUVASIMHE4HJXS7UEYY"
GOVERNANCE_FACTORY="CBUXBYLAQRUPGGYAZCIBC2B7X3G4RNRG2DFIHNW3UWVPGK2EO7ZWTNWB"

# Get admin address from identity
ADMIN_ADDRESS=$(stellar keys address "$ADMIN_IDENTITY")

echo -e "${BLUE}Step 1: Building all contracts...${NC}"
stellar contract build

if [ ! -d "target/wasm32v1-none/release" ]; then
    echo -e "${RED}Build failed! No WASM files found.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Contracts built successfully${NC}"
echo ""

# Function to upload WASM and get hash
upload_wasm() {
    local wasm_path=$1
    local contract_name=$2

    echo -e "${BLUE}Uploading ${contract_name}...${NC}" >&2

    # Use stellar contract upload with identity
    # The command outputs the hash on the last line (may skip if already uploaded)
    local output=$(stellar contract upload \
        --wasm "$wasm_path" \
        --source "$ADMIN_IDENTITY" \
        --network "$NETWORK" 2>&1)

    WASM_HASH=$(echo "$output" | grep -oE '[a-f0-9]{64}' | tail -1)

    if [ -z "$WASM_HASH" ]; then
        echo -e "${RED}❌ Failed to get hash for ${contract_name}${NC}" >&2
        echo "$output" >&2
        exit 1
    fi

    echo -e "${GREEN}✅ ${contract_name}: ${WASM_HASH}${NC}" >&2
    echo "$WASM_HASH"
}

echo -e "${BLUE}Step 2: Uploading WASMs to local network...${NC}"
echo ""

# Token contracts
ALLOWLIST_HASH=$(upload_wasm "target/wasm32v1-none/release/fungible_allowlist_example.wasm" "Allowlist Token")
BLOCKLIST_HASH=$(upload_wasm "target/wasm32v1-none/release/fungible_blocklist_example.wasm" "Blocklist Token")
CAPPED_HASH=$(upload_wasm "target/wasm32v1-none/release/fungible_capped_example.wasm" "Capped Token")
PAUSABLE_HASH=$(upload_wasm "target/wasm32v1-none/release/fungible_pausable_example.wasm" "Pausable Token")
VAULT_HASH=$(upload_wasm "target/wasm32v1-none/release/fungible_vault_example.wasm" "Vault Token")

# NFT contracts
ENUMERABLE_HASH=$(upload_wasm "target/wasm32v1-none/release/nft_enumerable_example.wasm" "Enumerable NFT")
ROYALTIES_HASH=$(upload_wasm "target/wasm32v1-none/release/nft_royalties_example.wasm" "Royalties NFT")
ACCESS_CONTROL_HASH=$(upload_wasm "target/wasm32v1-none/release/nft_access_control_example.wasm" "Access Control NFT")

# Governance contracts
MERKLE_VOTING_HASH=$(upload_wasm "target/wasm32v1-none/release/merkle_voting_example.wasm" "Merkle Voting")
# Note: Multisig contract will be added when available
# MULTISIG_HASH=$(upload_wasm "target/wasm32v1-none/release/multisig_example.wasm" "Multisig")

echo ""
echo -e "${BLUE}Step 3: Initializing Token Factory...${NC}"

stellar contract invoke \
    --id "$TOKEN_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_allowlist_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$ALLOWLIST_HASH"

stellar contract invoke \
    --id "$TOKEN_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_blocklist_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$BLOCKLIST_HASH"

stellar contract invoke \
    --id "$TOKEN_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_capped_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$CAPPED_HASH"

stellar contract invoke \
    --id "$TOKEN_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_pausable_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$PAUSABLE_HASH"

stellar contract invoke \
    --id "$TOKEN_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_vault_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$VAULT_HASH"

echo -e "${GREEN}✅ Token Factory initialized${NC}"
echo ""

echo -e "${BLUE}Step 4: Initializing NFT Factory...${NC}"

stellar contract invoke \
    --id "$NFT_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_enumerable_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$ENUMERABLE_HASH"

stellar contract invoke \
    --id "$NFT_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_royalties_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$ROYALTIES_HASH"

stellar contract invoke \
    --id "$NFT_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_access_control_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$ACCESS_CONTROL_HASH"

echo -e "${GREEN}✅ NFT Factory initialized${NC}"
echo ""

echo -e "${BLUE}Step 5: Initializing Governance Factory...${NC}"

stellar contract invoke \
    --id "$GOVERNANCE_FACTORY" \
    --source "$ADMIN_IDENTITY" \
    --network "$NETWORK" \
    -- set_merkle_voting_wasm \
    --admin "$ADMIN_ADDRESS" \
    --wasm_hash "$MERKLE_VOTING_HASH"

# Note: Multisig will be added when available
# stellar contract invoke \
#     --id "$GOVERNANCE_FACTORY" \
#     --source "$ADMIN_IDENTITY" \
#     --network "$NETWORK" \
#     -- set_multisig_wasm \
#     --admin "$ADMIN_ADDRESS" \
#     --wasm_hash "$MULTISIG_HASH"

echo -e "${GREEN}✅ Governance Factory initialized${NC}"
echo ""

echo -e "${GREEN}======================================"
echo "Local Network Setup Complete!"
echo "======================================${NC}"
echo ""
echo "WASM Hashes:"
echo "  Token Factory:"
echo "    - Allowlist: $ALLOWLIST_HASH"
echo "    - Blocklist: $BLOCKLIST_HASH"
echo "    - Capped: $CAPPED_HASH"
echo "    - Pausable: $PAUSABLE_HASH"
echo "    - Vault: $VAULT_HASH"
echo ""
echo "  NFT Factory:"
echo "    - Enumerable: $ENUMERABLE_HASH"
echo "    - Royalties: $ROYALTIES_HASH"
echo "    - Access Control: $ACCESS_CONTROL_HASH"
echo ""
echo "  Governance Factory:"
echo "    - Merkle Voting: $MERKLE_VOTING_HASH"
echo ""
echo "Factory addresses (local network):"
echo "  MasterFactory:     $MASTER_FACTORY"
echo "  TokenFactory:      $TOKEN_FACTORY"
echo "  NFTFactory:        $NFT_FACTORY"
echo "  GovernanceFactory: $GOVERNANCE_FACTORY"
echo ""
echo "Local addresses are ephemeral: they change whenever the Stellar Docker container"
echo "is recreated. Re-run this script and read the addresses above after every reset."
echo ""
echo "You can now deploy contracts on local network!"
