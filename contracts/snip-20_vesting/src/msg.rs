use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Uint128};

use crate::constants::ContractStatusLevel;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub owner: Option<HumanAddr>,
    pub contract_status: Option<u8>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {
        stage: Uint128
    },
    GetCurrentStage {},
    ContractOwner {},
    GrantedContractOwner {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    RegisterNewVestingRound {
        token_address: Option<HumanAddr>,
        is_paused: Option<bool>,
        merkle_root: String
    },
    SetContractStatus {
        level: ContractStatusLevel,
        // padding: Option<String>,
    },
    GrantContractOwner {
        new_admin: HumanAddr
    },
    ClaimContractOwner {},
    RevokeGrantedContractOwner {}
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct VestingRoundResponse {
    pub stage: Uint128,
    pub total_claimed: Uint128,
    pub token_address: String,
    pub merkle_root: String,
    pub created_at: u64
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ContractOwnerResponse {
    pub contract_owner: HumanAddr
}