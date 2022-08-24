use cosmwasm_std::{HumanAddr, StdError, Storage, Api, Querier, Extern};
use sha2::Digest;

use std::convert::{TryInto};
use crate::state::ReadonlyVestingRound;

pub fn verify_user_vesting_stats<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    proof: Vec<String>,
    user_addr: HumanAddr,
    stage: u128,
    total_amount: u128,
    tge: u128,
    start_at: u64,
    cliff: u64,
    duration: u64,
) -> Result<bool, StdError> {
    let user_input = format!(
        "{}{}{}{}{}{}{}",
        user_addr.to_string(),
        stage,
        total_amount,
        tge,
        start_at,
        duration,
        cliff
    );
    
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| StdError::generic_err("Wrong Length!"))?;
    
    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        let is_valid_merkle_proof = hex::decode_to_slice(p, &mut proof_buf);
    
        if !is_valid_merkle_proof.is_ok() {
            return Err(StdError::generic_err("Invalid merkle proof"));
        }
    
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| StdError::generic_err("Wrong Length!"))
    })?;
    
    let read_vesting_round = ReadonlyVestingRound::from_storage(&deps.storage);
    let merkle_root = read_vesting_round.config_by_stage(stage)?.merkle_root;
    
    let mut root_buf: [u8; 32] = [0; 32];
    let is_valid_merkle_root = hex::decode_to_slice(&merkle_root, &mut root_buf);
    
    if !is_valid_merkle_root.is_ok() {
        return Err(StdError::generic_err("Invalid merkle tree validation!"));
    }
    
    if root_buf != hash {
        return Err(StdError::generic_err("Proof verification failed!"));
    }

    Ok(true)
}   
