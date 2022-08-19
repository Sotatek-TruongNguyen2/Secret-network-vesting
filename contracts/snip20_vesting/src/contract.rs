use cosmwasm_std::{
    Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, CanonicalAddr, Uint128, HandleResult, InitResult, to_binary, debug_print
};

use crate::{msg::{HandleMsg, QueryMsg, InitMsg, ConfigResponse}, state::{Stage, Config, ConfigState, ReadonlyConfig, ReadonlyStage}};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let owner: Result<CanonicalAddr, StdError> = msg.owner.map_or(
        Ok(deps.api.canonical_address(&env.message.sender)?), 
        |addr| Ok(deps.api.canonical_address(&addr)?)
    );

    let merkle_root = &msg.merkle_root;

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    let is_valid_merkle_root = hex::decode_to_slice(&merkle_root, &mut root_buf);

    if !is_valid_merkle_root.is_ok() {
        return Err(StdError::generic_err("Invalid merkle tree validation!"));
    }

    let config = match msg.token_address {
        Some(token_address) => Ok(
            ConfigState {
                owner: owner?,
                merkle_root: merkle_root.clone(),
                token_address: deps.api.canonical_address(&token_address)?
            }
        ),
        None => Err(StdError::generic_err("Not a valid token address!"))
    }?;

    let mut stage_storage: Stage<S, Uint128> = Stage::from_storage(&mut deps.storage);
    let mut current_stage = stage_storage.current_stage().map_or(Uint128::zero(), |stage| stage);

    current_stage = Uint128::from(current_stage.0.checked_add(1).unwrap());

    stage_storage.save(&current_stage)?;

    let mut config_storage = Config::from_storage(&mut deps.storage);
    config_storage.make_config(current_stage, &config)?;
    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    Ok(HandleResponse::default())

    // match msg {
    //     HandleMsg::Increment {} => try_increment(deps, env),
    //     HandleMsg::Reset { count } => try_reset(deps, env, count),
    // }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig { stage } => to_binary(&get_config_by_stage(deps, stage.0)?),
        QueryMsg::GetCurrentStage {} => to_binary(&get_current_stage(deps)?)
    }
}

fn get_config_by_stage<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, stage: u128) -> StdResult<ConfigResponse> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let config = config_storage.config_by_stage(stage)?;
    Ok(ConfigResponse { 
        merkle_root: config.merkle_root,
        owner: deps.api.human_address(&config.owner)?.to_string(),
        token_address: deps.api.human_address(&config.token_address)?.to_string()
    })
}

fn get_current_stage<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Uint128> {
    let config_storage = ReadonlyStage::from_storage(&deps.storage);
    let current_stage = config_storage.current_stage()?;
    Ok(current_stage)
}