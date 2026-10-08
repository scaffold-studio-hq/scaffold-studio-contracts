//! # Merkle-Based Voting Contract Example
//!
//! This contract demonstrates how to implement a secure on-chain voting
//! mechanism using Merkle proofs.
//!
//! Eligible voters are encoded as leaf nodes in a Merkle tree, each containing:
//!
//! - `index: u32` — a unique identifier for the voter
//! - `account: Address` — the voter's address
//! - `voting_power: i128` — the weight of their vote
//!
//! To vote, users submit their `VoteData` and a Merkle proof verifying
//! inclusion in the tree. The contract ensures that each vote can only be cast
//! once (via the index) and tallies the result based on the `approve` flag.
//!
//! This pattern is useful for snapshot-based governance systems or off-chain
//! voter lists.
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address,
    BytesN, Env, Vec,
};
use stellar_contract_utils::{
    crypto::sha256::Sha256,
    merkle_distributor::{IndexableLeaf, MerkleDistributor},
};

type Distributor = MerkleDistributor<Sha256>;

#[contracttype]
#[derive(Clone)]
pub struct VoteData {
    pub index: u32,
    pub account: Address,
    /// Weight applied to the selected tally. Invariant: strictly positive;
    /// non-positive values are rejected before any tally mutation.
    pub voting_power: i128,
}

impl IndexableLeaf for VoteData {
    fn index(&self) -> u32 {
        self.index
    }
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum VoteError {
    InvalidVotingPower = 1,
    VoteTallyOverflow = 2,
}

#[contracttype]
pub enum DataKey {
    TotalVotesPro,
    TotalVotesAgainst,
}

#[contract]
pub struct MerkleVoting;

#[contractimpl]
impl MerkleVoting {
    pub fn __constructor(e: Env, root_hash: BytesN<32>) {
        Distributor::set_root(&e, root_hash);
        e.storage().instance().set(&DataKey::TotalVotesPro, &0i128);
        e.storage().instance().set(&DataKey::TotalVotesAgainst, &0i128);
    }

    pub fn vote(e: &Env, vote_data: VoteData, proof: Vec<BytesN<32>>, approve: bool) {
        // Reject non-positive voting power before the proof is consumed or any
        // tally is mutated: a negative weight would subtract from the opposing
        // total, and a zero weight wastes the leaf's single use.
        if vote_data.voting_power <= 0 {
            panic_with_error!(e, VoteError::InvalidVotingPower);
        }

        // Verify merkle proof using the MerkleDistributor
        Distributor::verify_and_set_claimed(e, vote_data.clone(), proof);

        // Update vote totals
        if approve {
            let current_pro: i128 = e.storage().instance().get(&DataKey::TotalVotesPro).unwrap();
            let new_pro = current_pro
                .checked_add(vote_data.voting_power)
                .unwrap_or_else(|| panic_with_error!(e, VoteError::VoteTallyOverflow));
            e.storage().instance().set(&DataKey::TotalVotesPro, &new_pro);
        } else {
            let current_against: i128 =
                e.storage().instance().get(&DataKey::TotalVotesAgainst).unwrap();
            let new_against = current_against
                .checked_add(vote_data.voting_power)
                .unwrap_or_else(|| panic_with_error!(e, VoteError::VoteTallyOverflow));
            e.storage()
                .instance()
                .set(&DataKey::TotalVotesAgainst, &new_against);
        }
    }

    pub fn has_voted(e: &Env, index: u32) -> bool {
        Distributor::is_claimed(e, index)
    }

    pub fn get_vote_results(e: Env) -> (i128, i128) {
        let votes_pro: i128 = e.storage().instance().get(&DataKey::TotalVotesPro).unwrap_or(0);
        let votes_against: i128 =
            e.storage().instance().get(&DataKey::TotalVotesAgainst).unwrap_or(0);
        (votes_pro, votes_against)
    }
}
