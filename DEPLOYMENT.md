# Stellar Studio - Local Deployment Reference

**Network**: Standalone Network ; February 2017
**RPC URL**: http://localhost:8000/rpc
**Account**: me
**Date**: November 4, 2025

---

## Factory Contracts (Deployed & Ready)

### 1. MasterFactory
**Contract ID**: `CCSDB3ONSEPAHAKUIAHNFYVAS4VYVFBI25PYXXLDWSOJOLKJFZJ2T6YB`
**WASM Hash**: `95cd7cdcfaefbdb11394e1c8ea5b302a406bf761da32b839582df751193a54b5`
**Admin**: me
**TypeScript Client**: ✅ Generated at `packages/master_factory`

**Functions**:
- `deploy_token_factory(deployer, wasm_hash, salt)` - Deploy TokenFactory
- `deploy_nft_factory(deployer, wasm_hash, salt)` - Deploy NFTFactory
- `deploy_governance_factory(deployer, wasm_hash, salt)` - Deploy GovernanceFactory
- `get_token_factory()` - Get TokenFactory address
- `get_nft_factory()` - Get NFTFactory address
- `get_governance_factory()` - Get GovernanceFactory address
- `get_deployed_factories()` - List all factories
- `get_admin()` - Get admin address
- `transfer_admin(current_admin, new_admin)` - Transfer admin role

---

### 2. TokenFactory
**Contract ID**: `CDELLSGENBMQ5U2O42TR5IEATP4HXCXOVOCFWLILL6GM57FYTWBUC6PO`
**WASM Hash**: `71528a97a2ae17a145fb456bfc06087390a281c105ebdd4b4ac7c289bd2f5782`
**Admin**: me
**TypeScript Client**: ✅ Generated at `packages/token_factory`

**Functions**:
- `set_allowlist_wasm(admin, wasm_hash)` - Set Allowlist token WASM
- `set_blocklist_wasm(admin, wasm_hash)` - Set Blocklist token WASM
- `set_capped_wasm(admin, wasm_hash)` - Set Capped token WASM
- `set_pausable_wasm(admin, wasm_hash)` - Set Pausable token WASM
- `set_vault_wasm(admin, wasm_hash)` - Set Vault token WASM
- `deploy_token(deployer, config)` - Deploy token with TokenConfig
- `get_deployed_tokens()` - List all deployed tokens
- `get_tokens_by_type(token_type)` - Filter by type
- `get_tokens_by_admin(admin)` - Filter by admin
- `get_token_count()` - Total count

---

### 3. NFTFactory
**Contract ID**: `CBIIXP5MEIHVVMJEVVASX2QYFUBVFXBHPS6DIG2XYNSDXPYBGBCIHJMY`
**WASM Hash**: `41e5918e5b314ecbe1b2693ad00ef5da3dce3ff01f0cecded75b7f56cc268bdd`
**Admin**: me
**TypeScript Client**: ✅ Generated at `packages/nft_factory`

**Functions**:
- `set_enumerable_wasm(admin, wasm_hash)` - Set Enumerable NFT WASM
- `set_royalties_wasm(admin, wasm_hash)` - Set Royalties NFT WASM
- `set_access_control_wasm(admin, wasm_hash)` - Set Access Control NFT WASM
- `deploy_nft(deployer, config)` - Deploy NFT with NFTConfig
- `get_deployed_nfts()` - List all deployed NFTs
- `get_nfts_by_type(nft_type)` - Filter by type
- `get_nfts_by_owner(owner)` - Filter by owner
- `get_nft_count()` - Total count

---

### 4. GovernanceFactory
**Contract ID**: `CCSZQKRZCBH47VKJPHDYE3FZBRF75TQ2B4IKHVQPFDGIPFYVTZVQCHGL`
**WASM Hash**: `6403fbb7e31f84978ada554b1777ee372aa59bdcb491b00ca9406a5a9e11a17a`
**Admin**: me
**TypeScript Client**: ✅ Generated at `packages/governance_factory`

**Functions**:
- `set_merkle_voting_wasm(admin, wasm_hash)` - Set Merkle Voting WASM
- `set_multisig_wasm(admin, wasm_hash)` - Set Multisig WASM
- `deploy_governance(deployer, config)` - Deploy governance with GovernanceConfig
- `get_deployed_governance()` - List all governance contracts
- `get_governance_by_type(governance_type)` - Filter by type
- `get_governance_by_admin(admin)` - Filter by admin
- `get_governance_count()` - Total count

---

## OpenZeppelin Contract WASM Hashes

These hashes are used by the factories to deploy contract instances programmatically.

### Token Contract WASMs

| Contract Type | WASM Hash | Use With |
|--------------|-----------|----------|
| **Allowlist Token** | `5fce8f47bc739541e9b6ec644895a15dc3a2154f9ee7dd58da91f448854888c0` | TokenFactory.set_allowlist_wasm() |
| **Blocklist Token** | `1171c13cb14989447e94ef634b0a5edad32c5e4cd16030a23d213e8c57b263f5` | TokenFactory.set_blocklist_wasm() |
| **Capped Token** | `20ae88d818a76af6d39f2f732b469936bdd23f868d937264e198d1ee6d1d699d` | TokenFactory.set_capped_wasm() |
| **Pausable Token** | `0c388f292170aff72d6d42ef7080cd924d00050172ff69d0d82b8984f9feb1b7` | TokenFactory.set_pausable_wasm() |
| **Vault Token** | `1e6850d877241dd8e27d5f5173a99b772c9891e1d2f71f5e5f4e8b0dd863b11f` | TokenFactory.set_vault_wasm() |

### NFT Contract WASMs

| Contract Type | WASM Hash | Use With |
|--------------|-----------|----------|
| **Enumerable NFT** | `cfcb55fb716f2c1d43f6970cb6eabbdf5b919fa9f21dc83eb4a7189966b46b6a` | NFTFactory.set_enumerable_wasm() |
| **Royalties NFT** | `48aa20c80a6216facd99eb3f778a82a4451844d289b197c5c1764b0e82b03bfb` | NFTFactory.set_royalties_wasm() |
| **Access Control NFT** | `7bcfa202d9bbc4d46820e0ac70ef2ed6fb69d0a96787aeb3b095b408b594a7b6` | NFTFactory.set_access_control_wasm() |

---

## Quick Start Commands

### Test Factory Contracts

```bash
# Get factory admin
stellar contract invoke --id master_factory -- get_admin

# Check if factories are deployed
stellar contract invoke --id master_factory -- get_token_factory
stellar contract invoke --id master_factory -- get_nft_factory
stellar contract invoke --id master_factory -- get_governance_factory

# Get deployed tokens/NFTs/governance
stellar contract invoke --id token_factory -- get_deployed_tokens
stellar contract invoke --id nft_factory -- get_deployed_nfts
stellar contract invoke --id governance_factory -- get_deployed_governance
```

### Configure TokenFactory (Example)

```bash
# Set Allowlist token WASM hash
stellar contract invoke \
  --id token_factory \
  -- \
  set_allowlist_wasm \
  --admin me \
  --wasm_hash 5fce8f47bc739541e9b6ec644895a15dc3a2154f9ee7dd58da91f448854888c0
```

### Deploy a Token (Example)

```bash
# Deploy an Allowlist token
stellar contract invoke \
  --id token_factory \
  -- \
  deploy_token \
  --deployer me \
  --config '{"token_type":"Allowlist","admin":"<address>","manager":"<address>","initial_supply":"1000000","cap":null,"salt":"<32-byte-salt>"}'
```

---

## Project Structure

```
packages/
├── master_factory/      ✅ TypeScript client
├── token_factory/       ✅ TypeScript client
├── nft_factory/         ✅ TypeScript client
└── governance_factory/  ✅ TypeScript client

target/stellar/local/
├── master_factory.wasm
├── token_factory.wasm
├── nft_factory.wasm
├── governance_factory.wasm
├── fungible_allowlist_example.wasm
├── fungible_blocklist_example.wasm
├── fungible_capped_example.wasm
├── fungible_pausable_example.wasm
├── fungible_vault_example.wasm
├── nft_enumerable_example.wasm
├── nft_royalties_example.wasm
└── nft_access_control_example.wasm
```

---

## Next Steps

1. **Configure Factories**: Set all WASM hashes in each factory
2. **Test Token Deployment**: Deploy test tokens using TokenFactory
3. **Test NFT Deployment**: Deploy test NFTs using NFTFactory
4. **Test Governance Deployment**: Deploy governance contracts using GovernanceFactory
5. **Verify Tracking**: Check that get_deployed_* functions return correct data

---

## Notes

- All factories are initialized with `admin = me`
- Local network must be running for contract invocations
- Use `stellar network start local` if network is down
- Use `stellar container logs local` to view Docker logs
- TypeScript clients are available in `packages/` directories

---

**Status**: ✅ ALL SYSTEMS OPERATIONAL
**Last Build**: November 4, 2025
**Total WASMs**: 13 contracts (4 factories + 9 OpenZeppelin)
**Total Factory Size**: 32.6KB
