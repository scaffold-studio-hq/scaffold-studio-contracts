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
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env, Vec};
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
    /// Weight added to the tally when this vote is cast. Must be strictly
    /// positive; [`MerkleVoting::vote`] rejects any value `<= 0`.
    pub voting_power: i128,
}

impl IndexableLeaf for VoteData {
    fn index(&self) -> u32 {
        self.index
    }
}

/// Errors returned by [`MerkleVoting`].
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MerkleVotingError {
    /// The supplied `VoteData.voting_power` was not strictly positive.
    InvalidVotingPower = 1,
    /// Adding the voting power overflowed the running tally.
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
        // Reject non-positive voting power before mutating any tally. The tally
        // accumulators are signed and the leaf is supplied by the caller, so a
        // `voting_power <= 0` entry would otherwise let a vote subtract from the
        // opposing total.
        if vote_data.voting_power <= 0 {
            panic_with_error!(e, MerkleVotingError::InvalidVotingPower);
        }

        // Verify merkle proof using the MerkleDistributor
        Distributor::verify_and_set_claimed(e, vote_data.clone(), proof);

        // Update vote totals
        if approve {
            let current_pro: i128 = e.storage().instance().get(&DataKey::TotalVotesPro).unwrap();
            let updated_pro = current_pro
                .checked_add(vote_data.voting_power)
                .unwrap_or_else(|| panic_with_error!(e, MerkleVotingError::VoteTallyOverflow));
            e.storage().instance().set(&DataKey::TotalVotesPro, &updated_pro);
        } else {
            let current_against: i128 =
                e.storage().instance().get(&DataKey::TotalVotesAgainst).unwrap();
            let updated_against = current_against
                .checked_add(vote_data.voting_power)
                .unwrap_or_else(|| panic_with_error!(e, MerkleVotingError::VoteTallyOverflow));
            e.storage()
                .instance()
                .set(&DataKey::TotalVotesAgainst, &updated_against);
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
