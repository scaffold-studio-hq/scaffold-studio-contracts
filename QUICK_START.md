# Stellar Studio Contracts - Quick Start Guide

Quick reference for building and deploying Stellar Studio contracts.

---

## Prerequisites

```bash
# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installations
stellar --version
rust --version
```

---

## Build Contracts

```bash
# Build all contracts
stellar contract build

# Output location
ls target/wasm32v1-none/release/*.wasm
```

---

## Local Development

### Start Local Network

```bash
# Start standalone network in Docker
stellar network start local

# Check status
docker ps | grep stellar
```

### Deploy to Local

```bash
# Run complete setup
./setup-local.sh

# This script will:
# - Build all contracts
# - Deploy factories
# - Initialize with WASM hashes
# - Display all contract addresses
```

### Important: Network Resets

When Docker restarts, all contracts are lost. Re-run setup:

```bash
# After Docker restart
./setup-local.sh
```

---

## Testnet Deployment

### One-Command Setup

```bash
# Create identity (first time only)
stellar keys generate me --network testnet

# Fund account
stellar keys fund me --network testnet

# Deploy everything
./setup-testnet.sh
```

### Factory Addresses (Testnet)

```
Token Factory:      CAHLJEQUCNTV7JPAPCMLCBIHOX7FFB57DUARJ6XGTW27FPCVKKY7JM2A
NFT Factory:        CDJQAGTVOK37NPBWMADBJDGFYM6BEAFV4T45S23D4LQLGSTMRRZ5RQ6X
Governance Factory: CC3SLHSCJHP7YJ462ZIACJ54VOHL5ZFUODZKBTITIZSO74D4YOPR5WCE
```

---

## Quick Test

### Query Factory

```bash
# Get token count
stellar contract invoke \
  --id CAHLJEQUCNTV7JPAPCMLCBIHOX7FFB57DUARJ6XGTW27FPCVKKY7JM2A \
  --source me \
  --network testnet \
  -- get_token_count
```

### Deploy Token

```bash
# Deploy pausable token.
# `salt` is 32 random bytes encoded as a 64-character hex string.
SALT=$(openssl rand -hex 32)

# The config is complete JSON. Generate a unique 32-byte salt for this deployment.
ACCOUNT="$(stellar keys address me)"
SALT_HEX="$(openssl rand -hex 32)"
stellar contract invoke \
  --id CAHLJEQUCNTV7JPAPCMLCBIHOX7FFB57DUARJ6XGTW27FPCVKKY7JM2A \
  --source me \
  --network testnet \
  -- deploy_token \
  --deployer $(stellar keys address me) \
  --config '{"token_type":{"tag":"Pausable"},"admin":"'$(stellar keys address me)'","manager":"'$(stellar keys address me)'","initial_supply":"1000000","cap":null,"name":"Test","symbol":"TST","decimals":7,"salt":"'"$SALT"'","asset":null,"decimals_offset":null}'
  --deployer "$ACCOUNT" \
  --config "$(printf '{"token_type":{"tag":"Pausable"},"admin":"%s","manager":"%s","name":"Test","symbol":"TST","decimals":7,"initial_supply":"1000000","salt":"%s","cap":null,"asset":null,"decimals_offset":null}' "$ACCOUNT" "$ACCOUNT" "$SALT_HEX")"
```

The tagged `token_type` enum matches the generated client shape. `salt` is a 64-character hex string representing 32 random bytes, and must be new for every deployment.

---

## Troubleshooting

### "Contract not found"

**Cause:** Local network was reset

**Fix:** Re-run `./setup-local.sh`

### "WASM not set"

**Cause:** Factory not initialized with WASM hashes

**Fix:** Run init script or set manually:
```bash
stellar contract invoke \
  --id <FACTORY_ID> \
  --source me \
  --network testnet \
  -- set_pausable_wasm \
  --admin $(stellar keys address me) \
  --wasm_hash <WASM_HASH>
```

### "Connection refused"

**Cause:** Local Stellar network not running

**Fix:** Start network:
```bash
stellar network start local
```

---

## Development Workflow

1. **Make changes** to contract code
2. **Build** - `stellar contract build`
3. **Test** - `cargo test`
4. **Deploy to local** - `./setup-local.sh`
5. **Test deployment** - Use CLI or frontend
6. **Deploy to testnet** - `./setup-testnet.sh`
7. **Verify** - Test on testnet
8. **Ready for mainnet** - After audit

---

## Additional Resources

- [README.md](./README.md) - Full documentation
- [DEPLOYMENT.md](./DEPLOYMENT.md) - Detailed deployment guide
- [Stellar Documentation](https://developers.stellar.org/)
