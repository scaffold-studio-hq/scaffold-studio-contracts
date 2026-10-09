use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    xdr::ToXdr,
    Address, BytesN, Env, IntoVal, Vec,
};
use stellar_contract_utils::crypto::{
    hashable::commutative_hash_pair, hasher::Hasher, sha256::Sha256,
};

use crate::contract::{MerkleVoting, MerkleVotingClient, VoteData};

fn hash_vote(e: &Env, data: &VoteData) -> BytesN<32> {
    let mut hasher = Sha256::new(e);
    hasher.update(data.clone().to_xdr(e));
    hasher.finalize()
}

/// Registers a two-leaf tree: `(vote1, proof1)` and `(vote2, proof2)` are the
/// valid voter/proof pairs, where `proof1` is the sibling leaf of `vote1`.
fn setup_tree(
    e: &Env,
) -> (
    MerkleVotingClient<'_>,
    VoteData,
    VoteData,
    Vec<BytesN<32>>,
    Vec<BytesN<32>>,
) {
    // Mock authorizations so each voter can authorize their own `vote` call.
    e.mock_all_auths();

    let voter1 = Address::generate(e);
    let voter2 = Address::generate(e);

    let vote1 = VoteData { index: 0, account: voter1, voting_power: 100 };
    let vote2 = VoteData { index: 1, account: voter2, voting_power: 50 };

    let leaf1 = hash_vote(e, &vote1);
    let leaf2 = hash_vote(e, &vote2);

    let root = commutative_hash_pair(&leaf1, &leaf2, Sha256::new(e));

    let contract_id = e.register(MerkleVoting, (root,));
    let client = MerkleVotingClient::new(e, &contract_id);

    // Each voter authorizes their own `vote` call.
    let proof1 = Vec::from_array(&e, [leaf2.clone()]);
    e.mock_auths(&[MockAuth {
        address: &voter1,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "vote",
            args: (vote1.clone(), proof1.clone(), true).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    client.vote(&vote1, &proof1, &true);

    let proof2 = Vec::from_array(&e, [leaf1.clone()]);
    e.mock_auths(&[MockAuth {
        address: &voter2,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "vote",
            args: (vote2.clone(), proof2.clone(), false).into_val(&e),
            sub_invokes: &[],
        },
    }]);
    let proof1 = Vec::from_array(e, [leaf2.clone()]);
    let proof2 = Vec::from_array(e, [leaf1.clone()]);

    (client, vote1, vote2, proof1, proof2)
}

#[test]
fn test_merkle_voting() {
    let e = Env::default();
    let (client, vote1, vote2, proof1, proof2) = setup_tree(&e);

    client.vote(&vote1, &proof1, &true);
    client.vote(&vote2, &proof2, &false);

    // Verify votes were recorded
    assert!(client.has_voted(&0));
    assert!(client.has_voted(&1));

    // Check vote results
    let (votes_pro, votes_against) = client.get_vote_results();
    assert_eq!(votes_pro, 100);
    assert_eq!(votes_against, 50);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_vote_requires_voter_auth() {
    let e = Env::default();

    let voter = Address::generate(&e);
    let impostor = Address::generate(&e);

    let vote = VoteData { index: 0, account: voter.clone(), voting_power: 100 };
    let leaf = hash_vote(&e, &vote);
    let root = leaf.clone();

    let contract_id = e.register(MerkleVoting, (root,));
    let client = MerkleVotingClient::new(&e, &contract_id);

    // Only the impostor authorizes; the voter named in the leaf never does.
    // `vote` begins with `vote_data.account.require_auth()`, so the host must
    // reject the call before any proof is verified.
    let proof: Vec<BytesN<32>> = Vec::new(&e);
    e.mock_auths(&[MockAuth {
        address: &impostor,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "vote",
            args: (vote.clone(), proof.clone(), true).into_val(&e),
            sub_invokes: &[],
        },
    }]);

    client.vote(&vote, &proof, &true);
}
#[should_panic(expected = "Error(Contract, #1)")]
fn test_vote_rejects_zero_voting_power() {
    let e = Env::default();

    let voter = Address::generate(&e);
    let vote = VoteData { index: 0, account: voter.clone(), voting_power: 0 };
    let leaf = hash_vote(&e, &vote);

    let contract_id = e.register(MerkleVoting, (leaf,));
    let client = MerkleVotingClient::new(&e, &contract_id);

    let proof = Vec::<BytesN<32>>::new(&e);
    client.vote(&vote, &proof, &true);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_vote_rejects_negative_voting_power() {
    let e = Env::default();

    let voter = Address::generate(&e);
    let vote = VoteData { index: 1, account: voter.clone(), voting_power: -5 };
    let leaf = hash_vote(&e, &vote);

    let contract_id = e.register(MerkleVoting, (leaf,));
    let client = MerkleVotingClient::new(&e, &contract_id);

    let proof = Vec::<BytesN<32>>::new(&e);
    client.vote(&vote, &proof, &false);
/// A voter cannot cast a second vote for an index that has already been
/// claimed: `Distributor::verify_and_set_claimed` rejects it with
/// `MerkleDistributorError::IndexAlreadyClaimed` (Contract, #1301).
#[test]
#[should_panic(expected = "Error(Contract, #1301)")]
fn test_double_vote_is_rejected() {
    let e = Env::default();
    let (client, vote1, _vote2, proof1, _proof2) = setup_tree(&e);

    client.vote(&vote1, &proof1, &true);
    // Re-submitting the exact same `VoteData` (same index) must panic.
    client.vote(&vote1, &proof1, &false);
}

/// A proof that does not verify against the stored root is rejected with
/// `MerkleDistributorError::InvalidProof` (Contract, #1302).
#[test]
#[should_panic(expected = "Error(Contract, #1302)")]
fn test_invalid_proof_is_rejected() {
    let e = Env::default();
    let (client, vote1, _vote2, _proof1, _proof2) = setup_tree(&e);

    // `proof1` should be the sibling leaf; hashing the leaf with itself does not
    // reproduce the root.
    let wrong_leaf = hash_vote(&e, &vote1);
    let wrong_proof = Vec::from_array(&e, [wrong_leaf]);
    client.vote(&vote1, &wrong_proof, &true);
}

/// A failed verification must not mark the index as claimed nor move the
/// tallies.
#[test]
fn test_failed_vote_leaves_state_unchanged() {
    let e = Env::default();
    let (client, vote1, _vote2, _proof1, _proof2) = setup_tree(&e);

    let wrong_leaf = hash_vote(&e, &vote1);
    let wrong_proof = Vec::from_array(&e, [wrong_leaf]);

    let result = client.try_vote(&vote1, &wrong_proof, &true);
    assert!(result.is_err());

    // The index is still unclaimed and no votes were tallied.
    assert!(!client.has_voted(&0));
    let (votes_pro, votes_against) = client.get_vote_results();
    assert_eq!(votes_pro, 0);
    assert_eq!(votes_against, 0);
}
