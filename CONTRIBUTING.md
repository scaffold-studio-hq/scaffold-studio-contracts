# Contributing to scaffold-studio-contracts

`scaffold-studio-contracts` is part of the [scaffold-studio-hq](https://github.com/scaffold-studio-hq) organisation and is
developed in the open. Contributions of every size are welcome.

## Finding work

Open tasks are published as bounty issues on the issue tracker:

**-> https://github.com/scaffold-studio-hq/scaffold-studio-contracts/issues**

- Each issue title carries its bounty, for example `[Bounty: $60] Add unit tests for ...`.
- If you are new to the codebase, start with issues labelled `good first issue`.
- Comment on the issue before you begin so it can be assigned to you.

## Repository layout

- `contracts/` — the Soroban/Rust workspace (13 crates: `master-factory`, `token-factory`,
  `nft-factory`, `governance-factory`, the `fungible-*` token implementations, the `nft-*`
  implementations and `merkle-voting`).
- `src/` — the React + Vite frontend that consumes the generated clients.
- `packages/` — TypeScript clients generated from the contracts.
- `setup-local.sh` / `setup-testnet.sh` — network setup + factory initialisation.

## Local commands

Rust contracts (run from the repository root):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test --package token-factory   # a single crate
stellar contract build               # optimised WASM build
```

Frontend (run from the repository root, Node.js 22+):

```bash
npm ci
npm run lint
npm run build
```

## Making the change

1. Fork the repository and branch from the default branch.
2. Keep the change scoped to the issue's acceptance criteria.
3. Run the existing test and lint commands before you commit.

## Opening a pull request

- Reference the issue in the description, for example `Closes #12`.
- One issue per pull request.
- Make sure CI passes before requesting review.

## Reporting a bug

Open an issue with steps to reproduce, the expected result and the actual result.

## Questions

Ask on the issue thread so the discussion stays alongside the task.
