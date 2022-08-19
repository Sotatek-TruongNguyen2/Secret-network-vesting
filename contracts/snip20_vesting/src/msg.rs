use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub owner: Option<HumanAddr>,
    pub token_address: Option<HumanAddr>,
    pub merkle_root: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {
        stage: Uint128
    },
    GetCurrentStage {}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    UpdateConfig {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        new_owner: Option<HumanAddr>,
        new_cw20_address: Option<HumanAddr>,
        new_native_token: Option<HumanAddr>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: String,
    pub token_address: String,
    pub merkle_root: String
}