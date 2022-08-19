use cosmwasm_std::{
    Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128, HandleResult, InitResult, to_binary, debug_print, HumanAddr, log, CanonicalAddr
};

use crate::{msg::{HandleMsg, QueryMsg, InitMsg, VestingRoundResponse}, state::{VestingRound, Config, ReadonlyConfig, VestingRoundState, ReadonlyVestingRound}, constants::{u8_to_status_level, ContractStatusLevel, status_level_to_u8}};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let owner = msg.owner.map_or(
        Ok(deps.api.canonical_address(&env.message.sender)?), 
        |addr| Ok(deps.api.canonical_address(&addr)?)
    )?;

    let contract_status = u8_to_status_level(msg.contract_status.map_or(
        0,
        |status| status
    ))?;

    let mut config_storage = Config::from_storage(&mut deps.storage);

    config_storage.set_contract_owner(&owner)?;
    config_storage.set_contract_status(contract_status)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    let contract_status = ReadonlyConfig::from_storage(&deps.storage).contract_status();

    match contract_status {
        ContractStatusLevel::StopAll => {
            let response = match msg {
                HandleMsg::SetContractStatus { level } => try_set_contract_status(deps, env, level),
                _ => Err(StdError::generic_err(
                    "This contract is stopped and this action is not allowed",
                )),
            };
            return response;
        }
        ContractStatusLevel::NormalRun => {} // If it's a normal run just continue
    }

    match msg {
        HandleMsg::RegisterNewVestingRound { merkle_root, token_address, is_paused} => try_register_new_round(deps, env, is_paused, token_address, merkle_root),
        HandleMsg::SetContractStatus { level } => try_set_contract_status(deps, env, level),
        HandleMsg::GrantContractOwner { new_admin } => try_transfer_contract_owner(deps, env, new_admin),
        HandleMsg::ClaimContractOwner { } => try_claim_contract_owner(deps, env),
        HandleMsg::RevokeGrantedContractOwner { } => try_revoke_granted_contract_owner(deps, env)
    }
}

// ================= Execution handler ===================

fn try_revoke_granted_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(
        &config_storage,
        &deps.api.canonical_address(&env.message.sender)?
    )?;

    let granted_contract_owner = config_storage.granted_contract_owner()?;

    if granted_contract_owner == CanonicalAddr::default() {
        return Err(StdError::generic_err("No granted contract owner existed!"));
    }

    config_storage.set_granted_contract_owner(&CanonicalAddr::default())?;
    
    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "revoke_granted_contract_owner"),
        ],
        data: None
    })
}

fn try_claim_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);
    let sender = deps.api.canonical_address(&env.message.sender)?;

    check_if_granted_admin(
        &config_storage,
        &sender
    )?;

    config_storage.set_contract_owner(&sender)?;
    config_storage.set_granted_contract_owner(&CanonicalAddr::default())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "claim_admin_ownership"),
            log("new_admin", env.message.sender),
        ],
        data: None
    })
}

fn try_transfer_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_admin: HumanAddr,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(
        &config_storage,
        &deps.api.canonical_address(&env.message.sender)?
    )?;

    config_storage.set_granted_contract_owner(&deps.api.canonical_address(&new_admin)?)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "transfer_admin_ownership"),
            log("admin", config_storage.contract_owner()?),
            log("granted_admin", new_admin),
            log("created_at", env.block.time)
        ],
        data: None
    })
}

fn try_set_contract_status<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    status_level: ContractStatusLevel,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(
        &config_storage,
        &deps.api.canonical_address(&env.message.sender)?
    )?;

    config_storage.set_contract_status(status_level.clone())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "set_contract_status"),
            log("status", "success"),
            log("contract_status", status_level_to_u8(status_level))
        ],
        data: None
    })
}

fn try_register_new_round<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    is_paused: Option<bool>,
    token_address: Option<HumanAddr>,
    merkle_root: String,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(&config_storage, &deps.api.canonical_address(&env.message.sender)?)?;

    let current_stage = config_storage.new_stage();
    
    let config = validate_round_config(
        deps,
        &env,
        token_address,
        is_paused,
        merkle_root
    )?;

    let mut vesting_round_storage = VestingRound::from_storage(&mut deps.storage);
    vesting_round_storage.make_config(current_stage, &config)?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "new_vesting_round"),
            log("status", "success"),
            log("token_address", deps.api.human_address(&config.token_address)?),
            log("merkle_tree", config.merkle_root),
            log("created_at", config.created_at),
            log("stage", current_stage)
        ],
        data: None,
    };

    Ok(res)
}

// ================= Utility function ===================

fn is_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<bool> {
    let owner = config.contract_owner()?;
    if & owner != account {
        return Ok(false);
    }

    Ok(true)
}

fn is_granted_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<bool> {
    let owner = config.granted_contract_owner()?;
    if & owner != account {
        return Ok(false);
    }

    Ok(true)
}

fn check_if_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<()> {
    if !is_admin(config, account)? {
        return Err(StdError::generic_err(
            "This is an admin command. Admin commands can only be run from admin address",
        ));
    }

    Ok(())
}

fn check_if_granted_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<()> {
    if !is_granted_admin(config, account)? {
        return Err(StdError::generic_err(
            "This is a granted admin command. Granted admin commands can only be run from granted admin address",
        ));
    }

    Ok(())
}

fn validate_round_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
    token_address: Option<HumanAddr>,
    is_paused: Option<bool>,
    merkle_root: String
) -> StdResult<VestingRoundState> {
    let is_paused = is_paused.map_or(
        false,
        |paused| paused
    );

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    let is_valid_merkle_root = hex::decode_to_slice(&merkle_root, &mut root_buf);

    if !is_valid_merkle_root.is_ok() {
        return Err(StdError::generic_err("Invalid merkle tree validation!"));
    }

    let config = match token_address {
        Some(token_address) => Ok(
            VestingRoundState {
                created_at: env.block.time,
                merkle_root,
                is_paused,
                token_address: deps.api.canonical_address(&token_address)?,
                total_claimed: Uint128::zero()
            }
        ),
        None => Err(StdError::generic_err("Not a valid token address!"))
    }?;

    Ok(config)
}

// ================= Query ===================

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig { stage } => to_binary(&get_config_by_stage(deps, stage.0)?),
        QueryMsg::GetCurrentStage {} => to_binary(&get_current_stage(deps)?),
        QueryMsg::ContractOwner {} => to_binary(&get_contract_owner(deps)?),
        QueryMsg::GrantedContractOwner {} => to_binary(&get_granted_contract_owner(deps)?)
    }
}

fn get_config_by_stage<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, stage: u128) -> StdResult<VestingRoundResponse> {
    let config_storage = ReadonlyVestingRound::from_storage(&deps.storage);
    let config = config_storage.config_by_stage(stage)?;
    Ok(VestingRoundResponse { 
        stage: Uint128::from(stage),
        total_claimed: config.total_claimed,
        merkle_root: config.merkle_root,
        token_address: deps.api.human_address(&config.token_address)?.to_string(),
        created_at: config.created_at
    })
}

fn get_current_stage<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Uint128> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let current_stage = Uint128::from(config_storage.current_stage());
    Ok(current_stage)
}

fn get_contract_owner<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<HumanAddr> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let contract_owner = config_storage.contract_owner()?;
    Ok(deps.api.human_address(&contract_owner)?)
}

fn get_granted_contract_owner<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<HumanAddr> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let contract_owner = config_storage.granted_contract_owner()?;
    Ok(deps.api.human_address(&contract_owner)?)
}