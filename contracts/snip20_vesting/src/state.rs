use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::any::type_name;


use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage, ReadonlySingleton, Singleton};
use secret_toolkit::{
    storage::{TypedStoreMut, TypedStore},
};

use crate::helpers::set_bin_data;
use crate::utils::Duration;

pub static PREFIX_STAGE: &[u8] = b"stage";
pub static PREFIX_CONFIG: &[u8] = b"config";
pub static USER_VESTING_STATS_PREFIX: &[u8] = b"user_vesting";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigState {
    /// Owner If None set, contract is frozen.
    pub owner: CanonicalAddr,
    pub token_address: CanonicalAddr,
    pub merkle_root: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserVestingStatsState {
    /// Owner If None set, contract is frozen.
    pub user: CanonicalAddr,
    pub total_amount: Uint128,
    pub total_claimed: Uint128,
    pub start_vesting_epoch: Duration,
    pub vesting_duration: Duration,
    pub cliff: Duration,
    pub tge: Uint128,
    pub last_claim_epoch: Uint128,
}

// ============== Config (Mutate ) ================= //

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

    pub fn config_by_stage(&self, current_stage: u128) -> StdResult<ConfigState> {
        self.as_readonly().config(current_stage)
    }

    pub fn make_config(&mut self, stage: Uint128, config: &ConfigState) -> StdResult<()> {
        set_bin_data(&mut self.storage, &stage.0.to_be_bytes(), &config)
    }
}

// ============== Config ( Readonly - Implement ) ================= //

pub struct ReadonlyConfig<'a, S: ReadonlyStorage> {
    storage: ReadonlyPrefixedStorage<'a, S>,
}

impl<'a, S: ReadonlyStorage> ReadonlyConfig<'a, S> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlyPrefixedStorage::new(PREFIX_CONFIG, storage),
        }
    }

    pub fn config_by_stage(&self, current_stage: u128) -> StdResult<ConfigState> {
        self.as_readonly().config(current_stage)
    }

    fn as_readonly(&self) -> ReadonlyConfigImpl<ReadonlyPrefixedStorage<S>> {
        ReadonlyConfigImpl(&self.storage)
    }
}

struct ReadonlyConfigImpl<'a, S: ReadonlyStorage>(&'a S);

impl<'a, S: ReadonlyStorage> ReadonlyConfigImpl<'a, S> {
    fn config(&self, current_stage: u128) -> StdResult<ConfigState> {
        let consts_bytes = self
            .0
            .get(&current_stage.to_be_bytes())
            .ok_or_else(|| StdError::generic_err("No configuration for this stage"))?;
        bincode2::deserialize::<ConfigState>(&consts_bytes)
            .map_err(|e| StdError::serialize_err(type_name::<ConfigState>(), e))
    }
}

// ============== Stage (Mutate - ReadOnly) ================= //

pub struct Stage<'a, S: Storage, T: Serialize + DeserializeOwned> {
    storage: Singleton<'a, S, T>,
}

impl<'a, S: Storage, T: Serialize + DeserializeOwned> Stage<'a, S, T> {
    pub fn from_storage(storage: &'a mut S) -> Self {
        Self {
            storage: Singleton::new(storage, PREFIX_STAGE),
        }
    }

    pub fn current_stage(&mut self) -> StdResult<T> {
        self.storage.load()
    }

    pub fn save(&mut self, val: &T) -> StdResult<()> {
        self.storage.save(val)
    }
}

pub struct ReadonlyStage<'a, S: Storage, T: Serialize + DeserializeOwned> {
    storage: ReadonlySingleton<'a, S, T>,
}

impl<'a, S: Storage, T: Serialize + DeserializeOwned> ReadonlyStage<'a, S, T> {
    pub fn from_storage(storage: &'a S) -> Self {
        Self {
            storage: ReadonlySingleton::new(storage, PREFIX_STAGE),
        }
    }

    pub fn current_stage(&self) -> StdResult<T> {
        self.storage.load()
    }
}

// ============== User Vesting (Mutate ) ================= //

pub fn write_user_vesting_stats<S: Storage>(
    storage: &mut S,
    vesting_stats: &UserVestingStatsState,
    stage: Uint128
) -> StdResult<()> {
    let mut user_vesting_store = PrefixedStorage::multilevel(
        &[USER_VESTING_STATS_PREFIX, vesting_stats.user.as_slice()],
        storage,
    );
    let mut user_vesting_store = TypedStoreMut::attach(&mut user_vesting_store);

    user_vesting_store.store(&stage.0.to_be_bytes(), vesting_stats)
}

pub fn read_user_vesting_stats<S: Storage>(
    storage: &mut S,
    user: &CanonicalAddr,
    stage: Uint128
) -> StdResult<Option<UserVestingStatsState>> {
    let user_vesting_store = ReadonlyPrefixedStorage::multilevel(
        &[USER_VESTING_STATS_PREFIX, user.as_slice()],
        storage,
    );
    let user_vesting_store: TypedStore<UserVestingStatsState, ReadonlyPrefixedStorage<S>> = TypedStore::attach(&user_vesting_store);

    user_vesting_store.may_load(&stage.0.to_be_bytes())
}
