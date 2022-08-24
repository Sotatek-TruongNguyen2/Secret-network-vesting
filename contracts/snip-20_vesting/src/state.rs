use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::any::type_name;


use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage, Uint128 };
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage, Bucket, ReadonlyBucket};

use crate::{helpers::{set_bin_data, slice_to_u128, slice_to_u8}, constants::{ContractStatusLevel, status_level_to_u8, u8_to_status_level}};

pub static PREFIX_CONTRACT_OWNER_GRANTED: &[u8] = b"contract_owner_granted";
pub static PREFIX_CONTRACT_OWNER: &[u8] = b"contract_owner";
pub static PREFIX_CONTRACT_STATUS: &[u8] = b"contract_status";
pub static PREFIX_STAGE: &[u8] = b"stage";
pub static PREFIX_CONFIG: &[u8] = b"config";
pub static PREFIX_VESTING_ROUND: &[u8] = b"vesting_round";
pub static USER_VESTING_STATS_PREFIX: &[u8] = b"user_vesting";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VestingRoundState {
    /// Owner If None set, contract is frozen
    pub distribution: CanonicalAddr,
    pub token_code_hash: String,
    pub token_address: CanonicalAddr,
    pub total_claimed: Uint128,
    pub merkle_root: String,
    pub created_at: u64,
    pub is_paused: bool
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserVestingStatsState {
    /// Owner If None set, contract is frozen.
    pub user: CanonicalAddr,
    pub total_amount: Uint128,
    pub total_claimed: Uint128,
    pub vesting_duration: u64,
    pub cliff: u64,
    pub tge: Uint128,
    pub start_vesting_epoch: u64,
    // pub next_claim_epoch: u64,
    pub next_claim_epoch_index: u64
}

// ============== VestingRound (Mutate ) ================= //

pub struct VestingRound<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> VestingRound<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(PREFIX_VESTING_ROUND, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyVestingRoundImpl<PrefixedStorage<S>> {
        ReadonlyVestingRoundImpl(&self.storage)
    }

    pub fn config_by_stage(&self, current_stage: u128) -> StdResult<VestingRoundState> {
        self.as_readonly().config(current_stage)
    }

    pub fn make_config(&mut self, stage: u128, config: &VestingRoundState) -> StdResult<()> {
        set_bin_data(&mut self.storage, &stage.to_be_bytes(), &config)
    }
}

// ============== VestingRound ( Readonly - Implement ) ================= //

pub struct ReadonlyVestingRound<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyVestingRound<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(PREFIX_VESTING_ROUND, storage),
        }
    }

    pub fn config_by_stage(&self, current_stage: u128) -> StdResult<VestingRoundState> {
        self.as_readonly().config(current_stage)
    }

    fn as_readonly(&self) -> ReadonlyVestingRoundImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyVestingRoundImpl(&self.storage)
    }
}

struct ReadonlyVestingRoundImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyVestingRoundImpl<'a, S> {
    fn config(&self, current_stage: u128) -> StdResult<VestingRoundState> {
        let consts_bytes = self
            .0
            .get(&current_stage.to_be_bytes())
            .ok_or_else(|| StdError::generic_err("No configuration for this stage"))?;
        bincode2::deserialize::<VestingRoundState>(&consts_bytes)
            .map_err(|e| StdError::serialize_err(type_name::<VestingRoundState>(), e))
    }
}

// ============== SYSTEM CONFIG (Mutate) ================= //

pub struct Config<'a, S: Storage> {
    storage: PrefixedStorage<'a, S>,
}

impl<'a, S: Storage> Config<'a, S> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: PrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<PrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn current_stage(&self) -> u128 {
        self.as_readonly().current_stage()
    }

    pub fn contract_status(&self) -> ContractStatusLevel {
        self.as_readonly().contract_status()
    }

    pub fn contract_owner(&self) -> StdResult<CanonicalAddr> {
        self.as_readonly().contract_owner()
    }

    pub fn granted_contract_owner(&self) -> StdResult<CanonicalAddr> {
        self.as_readonly().granted_contract_owner()
    }

    pub fn new_stage(&mut self) -> u128 {
        let mut current_stage = self.storage.get(PREFIX_STAGE).map_or(
            0,
            |stage_bytes| slice_to_u128(&stage_bytes).unwrap()
        );

        current_stage = current_stage.checked_add(1).unwrap();
        self.storage.set(PREFIX_STAGE, &current_stage.to_be_bytes());
        
        current_stage
    }

    pub fn set_contract_status(&mut self, status: ContractStatusLevel) -> StdResult<()> {
        self.storage.set(PREFIX_CONTRACT_STATUS, &status_level_to_u8(status).to_be_bytes());
        Ok(())
    }

    pub fn set_contract_owner(&mut self, owner: &CanonicalAddr) -> StdResult<()> {
        set_bin_data(&mut self.storage, PREFIX_CONTRACT_OWNER, &owner)
    }

    pub fn set_granted_contract_owner(&mut self, owner: &CanonicalAddr) -> StdResult<()> {
        set_bin_data(&mut self.storage, PREFIX_CONTRACT_OWNER_GRANTED, &owner)
    }
 }


// ============== SYSTEM CONFIG ( Readonly - Implement ) ================= //

pub struct ReadonlyConfig<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyConfig<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }

    pub fn contract_owner(&self) -> StdResult<CanonicalAddr> {
        self.as_readonly().contract_owner()
    }

    pub fn granted_contract_owner(&self) -> StdResult<CanonicalAddr> {
        self.as_readonly().granted_contract_owner()
    }

    pub fn current_stage(&self) -> u128 {
        self.as_readonly().current_stage()
    }

    pub fn contract_status(&self) -> ContractStatusLevel {
        self.as_readonly().contract_status()
    }
}

struct ReadonlyConfigImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyConfigImpl<'a, S> {
    fn current_stage(&self) -> u128 {
        let stage_bytes = self
            .0
            .get(PREFIX_STAGE)
            .expect("no stage stored in config");
        
        slice_to_u128(&stage_bytes).unwrap()
    }

    fn contract_status(&self) -> ContractStatusLevel {
        let stage_bytes = self
            .0
            .get(PREFIX_CONTRACT_STATUS)
            .expect("no contract status stored in config");
        
        let status = slice_to_u8(&stage_bytes).unwrap();
        u8_to_status_level(status).unwrap()
    }

    fn contract_owner(&self) -> StdResult<CanonicalAddr> {
        let contract_owner_bytes = self
            .0
            .get(PREFIX_CONTRACT_OWNER)
            .ok_or_else(|| StdError::generic_err("no contract owner stored in configuration"))?;
            bincode2::deserialize::<CanonicalAddr>(&contract_owner_bytes)
                .map_err(|e| StdError::serialize_err(type_name::<CanonicalAddr>(), e))
    }

    fn granted_contract_owner(&self) -> StdResult<CanonicalAddr> {
        let contract_owner_bytes = self
            .0
            .get(PREFIX_CONTRACT_OWNER_GRANTED)
            .ok_or_else(|| StdError::generic_err("no granted contract owner stored in configuration"))?;
            bincode2::deserialize::<CanonicalAddr>(&contract_owner_bytes)
                .map_err(|e| StdError::serialize_err(type_name::<CanonicalAddr>(), e))
    }
}

// ============== User Vesting (Mutate ) ================= //

pub fn write_user_vesting_stats<S: Storage>(
    storage: &mut S,
    vesting_stats: &UserVestingStatsState,
    stage: u128
) -> StdResult<()> {
    let mut user_vesting_store = Bucket::<S, UserVestingStatsState>::multilevel(
        &[USER_VESTING_STATS_PREFIX, vesting_stats.user.as_slice()],
        storage,
    );

    user_vesting_store.save(&stage.to_be_bytes(), vesting_stats)
}

pub fn read_user_vesting_stats<S: Storage>(
    storage: &S,
    user: &CanonicalAddr,
    stage: u128
) -> StdResult<Option<UserVestingStatsState>> {
    let user_vesting_store = ReadonlyBucket::<S, UserVestingStatsState>::multilevel(
        &[USER_VESTING_STATS_PREFIX, user.as_slice()],
        storage,
    );

   user_vesting_store.may_load(&stage.to_be_bytes())
}
