use cosmwasm_std::{
    debug_print, log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, InitResponse, InitResult, Querier, StdError, StdResult, Storage,
    Uint128,
};
use secret_toolkit::snip20;

use crate::{
    constants::{
        status_level_to_u8, u8_to_status_level, ContractStatusLevel, ONE_DAY_IN_SECONDS,
        TGE_PRECISION,
    },
    merkle_proof::vesting_stats_verify::verify_user_vesting_stats,
    msg::{HandleMsg, InitMsg, QueryMsg, VestingRoundResponse},
    state::{
        read_user_vesting_stats, write_user_vesting_stats, Config, ReadonlyConfig,
        ReadonlyVestingRound, UserVestingStatsState, VestingRound, VestingRoundState,
    },
    vesting::calc_vesting_schedule::calc_current_vesting_amount,
};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let owner = msg.owner.map_or(
        Ok(deps.api.canonical_address(&env.message.sender)?),
        |addr| Ok(deps.api.canonical_address(&addr)?),
    )?;

    let contract_status = u8_to_status_level(msg.contract_status.map_or(0, |status| status))?;

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
        HandleMsg::RegisterNewVestingRound {
            merkle_root,
            token_address,
            is_paused,
            token_code_hash,
            distribution,
        } => try_register_new_round(
            deps,
            env,
            is_paused,
            distribution,
            token_address,
            token_code_hash,
            merkle_root,
        ),
        HandleMsg::SetContractStatus { level } => try_set_contract_status(deps, env, level),
        HandleMsg::GrantContractOwner { new_admin } => {
            try_transfer_contract_owner(deps, env, new_admin)
        }
        HandleMsg::ClaimContractOwner {} => try_claim_contract_owner(deps, env),
        HandleMsg::RevokeGrantedContractOwner {} => try_revoke_granted_contract_owner(deps, env),
        HandleMsg::Claim {
            proof,
            stage,
            amount,
            tge,
            start_at,
            duration,
            cliff,
        } => try_claim(
            deps,
            env,
            proof,
            stage.u128(),
            amount.u128(),
            tge.u128(),
            start_at,
            cliff,
            duration,
        ),
    }
}

// ================= Execution handler ===================

fn try_claim<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proof: Vec<String>,
    stage: u128,
    total_amount: u128,
    tge: u128,
    start_at: u64,
    cliff: u64,
    duration: u64,
) -> StdResult<HandleResponse> {
    let mut output_msgs = vec![];
    let mut logs = vec![];

    verify_user_vesting_stats(
        deps,
        proof,
        env.message.sender.clone(),
        stage,
        total_amount,
        tge,
        start_at,
        cliff,
        duration,
    )?;

    let user_vesting_stats = read_user_vesting_stats(
        &deps.storage,
        &deps.api.canonical_address(&env.message.sender)?,
        stage,
    )?;
    let mut config_storage = VestingRound::from_storage(&mut deps.storage);
    let mut config = config_storage.config_by_stage(stage)?;

    // Check if vesting time already started or still in pending state
    if start_at.gt(&env.block.time) {
        return Err(StdError::generic_err("Claim time have not started yet!"));
    }

    // Create data for user and pay TGE for first time vesting
    let mut user_vesting_stats = match user_vesting_stats {
        Some(user_vesting_stats) => user_vesting_stats,
        _ => {
            let mut user_vesting_stats = UserVestingStatsState {
                tge: Uint128::from(tge),
                cliff,
                next_claim_epoch_index: start_at
                    .checked_add(cliff)
                    .unwrap()
                    .checked_div(ONE_DAY_IN_SECONDS)
                    .unwrap()
                    .checked_add(1u64)
                    .unwrap(),
                total_amount: Uint128::from(total_amount),
                total_claimed: Uint128::from(0u128),
                user: deps.api.canonical_address(&env.message.sender)?,
                start_vesting_epoch: start_at,
                vesting_duration: duration,
            };

            if tge.gt(&0u128) {
                let tge_amount = total_amount
                    .checked_mul(tge)
                    .unwrap()
                    .checked_div(TGE_PRECISION)
                    .unwrap();

                output_msgs.push(snip20::transfer_from_msg(
                    deps.api.human_address(&config.distribution)?,
                    env.message.sender.clone(),
                    Uint128::from(tge_amount),
                    Some(String::from("Pay TGE")),
                    None,
                    256,
                    config.token_code_hash.clone(),
                    deps.api.human_address(&config.token_address.clone())?,
                )?);

                user_vesting_stats.total_claimed = Uint128::from(
                    user_vesting_stats
                        .total_claimed
                        .u128()
                        .checked_add(tge_amount)
                        .unwrap(),
                );
                user_vesting_stats.total_amount = Uint128::from(
                    user_vesting_stats
                        .total_amount
                        .u128()
                        .checked_sub(tge_amount)
                        .unwrap(),
                );

                config.total_claimed =
                    Uint128::from(config.total_claimed.u128().checked_add(tge_amount).unwrap());
            
                logs.push(log("tge_amount", tge_amount));
            }

            user_vesting_stats
        }
    };

    // Check if current time passed over cliff period
    let is_cliff_passed = env.block.time.gt(&user_vesting_stats
        .start_vesting_epoch
        .checked_add(user_vesting_stats.cliff)
        .unwrap());

        
    // check whether there exists remaining tokens amount to claim
    if user_vesting_stats.total_claimed >= user_vesting_stats.total_amount {
        return Err(StdError::generic_err("Exceeds maximum claim amount!"));
    }

    // Check if cliff period is already passed or not
    match is_cliff_passed {
        true => {
            let current_epoch_index = env.block.time.checked_div(ONE_DAY_IN_SECONDS).unwrap();

            // Check if routine claim is already vested
            if current_epoch_index < user_vesting_stats.next_claim_epoch_index {
                return Err(StdError::generic_err("Routine claim is already vested!"));
            }

            // Calculate claim amount by daily
            let (claim_amount, next_claim_epoch_index) = calc_current_vesting_amount(
                current_epoch_index,
                user_vesting_stats.next_claim_epoch_index,
                user_vesting_stats.vesting_duration,
                user_vesting_stats.total_amount.u128(),
            )?;

            if claim_amount.gt(&0u128) {
                output_msgs.push(snip20::transfer_from_msg(
                    deps.api.human_address(&config.distribution)?,
                    env.message.sender.clone(),
                    Uint128::from(claim_amount),
                    Some(String::from("Pay Claim amount")),
                    None,
                    256,
                    config.token_code_hash.clone(),
                    deps.api.human_address(&config.token_address.clone())?,
                )?);

                user_vesting_stats.next_claim_epoch_index = next_claim_epoch_index;
                user_vesting_stats.total_claimed = Uint128::from(
                    user_vesting_stats
                        .total_claimed
                        .u128()
                        .checked_add(claim_amount)
                        .unwrap(),
                );

                config.total_claimed = Uint128::from(
                    config
                        .total_claimed
                        .u128()
                        .checked_add(claim_amount)
                        .unwrap(),
                );

                logs.push(log("claim_amount", claim_amount));
                logs.push(log("claim_at", env.block.time));
            }
        }
        _ => {
            if user_vesting_stats.tge.u128().gt(&0u128) {
                return Err(StdError::generic_err("Cliff time not passed!"));
            }
            // match user_vesting_stats.tge.u128().gt(&0u128) {
            //     true => Err(StdError::generic_err("Cliff time not passed!")),
            //     _ => Ok(()),
            // }?;
        }
    };

    config_storage.make_config(stage, &config)?;
    write_user_vesting_stats(&mut deps.storage, &user_vesting_stats, stage)?;

    let res = HandleResponse {
        messages: output_msgs,
        log: [vec![log("event", "claim"), log("status", "success")], logs].concat(),
        data: None,
    };

    Ok(res)
}

fn try_revoke_granted_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(
        &config_storage,
        &deps.api.canonical_address(&env.message.sender)?,
    )?;

    let granted_contract_owner = config_storage.granted_contract_owner()?;

    if granted_contract_owner == CanonicalAddr::default() {
        return Err(StdError::generic_err("No granted contract owner existed!"));
    }

    config_storage.set_granted_contract_owner(&CanonicalAddr::default())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("event", "revoke_granted_contract_owner")],
        data: None,
    })
}

fn try_claim_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let mut config_storage = Config::from_storage(&mut deps.storage);
    let sender = deps.api.canonical_address(&env.message.sender)?;

    check_if_granted_admin(&config_storage, &sender)?;

    config_storage.set_contract_owner(&sender)?;
    config_storage.set_granted_contract_owner(&CanonicalAddr::default())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "claim_admin_ownership"),
            log("new_admin", env.message.sender),
        ],
        data: None,
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
        &deps.api.canonical_address(&env.message.sender)?,
    )?;

    config_storage.set_granted_contract_owner(&deps.api.canonical_address(&new_admin)?)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "transfer_admin_ownership"),
            log("admin", config_storage.contract_owner()?),
            log("granted_admin", new_admin),
            log("created_at", env.block.time),
        ],
        data: None,
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
        &deps.api.canonical_address(&env.message.sender)?,
    )?;

    config_storage.set_contract_status(status_level.clone())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("event", "set_contract_status"),
            log("status", "success"),
            log("contract_status", status_level_to_u8(status_level)),
        ],
        data: None,
    })
}

fn try_register_new_round<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    is_paused: Option<bool>,
    distribution: Option<HumanAddr>,
    token_address: Option<HumanAddr>,
    token_code_hash: Option<String>,
    merkle_root: String,
) -> StdResult<HandleResponse> {
    let mut output_msgs = vec![];
    let mut config_storage = Config::from_storage(&mut deps.storage);

    check_if_admin(
        &config_storage,
        &deps.api.canonical_address(&env.message.sender)?,
    )?;

    let current_stage = config_storage.new_stage();

    let config = validate_round_config(
        deps,
        &env,
        distribution,
        token_address,
        token_code_hash,
        is_paused,
        merkle_root,
    )?;

    let mut vesting_round_storage = VestingRound::from_storage(&mut deps.storage);
    vesting_round_storage.make_config(current_stage, &config)?;

    let callback_contract_addr = deps.api.human_address(&config.token_address)?;

    output_msgs.push(snip20::register_receive_msg(
        env.contract_code_hash.clone(),
        None,
        256,
        config.token_code_hash.clone(),
        callback_contract_addr.clone(),
    )?);

    output_msgs.push(snip20::set_viewing_key_msg(
        "Snip20-vesting".into(),
        None,
        256,
        config.token_code_hash.clone(),
        callback_contract_addr.clone(),
    )?);

    let res = HandleResponse {
        messages: output_msgs,
        log: vec![
            log("event", "new_vesting_round"),
            log("status", "success"),
            log("token_code_hash", config.token_code_hash),
            log("token_address", callback_contract_addr),
            log("merkle_tree", config.merkle_root),
            log("created_at", config.created_at),
            log("stage", current_stage),
        ],
        data: None,
    };

    Ok(res)
}

// ================= Utility function ===================

fn is_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<bool> {
    let owner = config.contract_owner()?;
    if &owner != account {
        return Ok(false);
    }

    Ok(true)
}

fn is_granted_admin<S: Storage>(config: &Config<S>, account: &CanonicalAddr) -> StdResult<bool> {
    let owner = config.granted_contract_owner()?;
    if &owner != account {
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

fn check_if_granted_admin<S: Storage>(
    config: &Config<S>,
    account: &CanonicalAddr,
) -> StdResult<()> {
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
    distribution: Option<HumanAddr>,
    token_address: Option<HumanAddr>,
    token_code_hash: Option<String>,
    is_paused: Option<bool>,
    merkle_root: String,
) -> StdResult<VestingRoundState> {
    let is_paused = is_paused.map_or(false, |paused| paused);

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    let is_valid_merkle_root = hex::decode_to_slice(&merkle_root, &mut root_buf);

    if !is_valid_merkle_root.is_ok() {
        return Err(StdError::generic_err("Invalid merkle tree validation!"));
    }

    // Specify distribution address
    let distribution_addr = deps.api.canonical_address(
        &distribution.map_or(Ok(env.message.sender.clone()), |addr| Ok(addr))?,
    )?;

    let config = match (token_address, token_code_hash) {
        (Some(token_address), Some(token_code_hash)) => Ok(VestingRoundState {
            distribution: distribution_addr,
            created_at: env.block.time,
            merkle_root,
            is_paused,
            token_address: deps.api.canonical_address(&token_address)?,
            token_code_hash,
            total_claimed: Uint128::zero(),
        }),
        (Some(_), None) => Err(StdError::generic_err("Not a valid token code hash!")),
        (None, Some(_)) => Err(StdError::generic_err("Not a valid token address!")),
        _ => Err(StdError::generic_err(
            "Invalid both token address and token code hash!",
        )),
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
        QueryMsg::GrantedContractOwner {} => to_binary(&get_granted_contract_owner(deps)?),
    }
}

fn get_config_by_stage<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    stage: u128,
) -> StdResult<VestingRoundResponse> {
    let config_storage = ReadonlyVestingRound::from_storage(&deps.storage);
    let config = config_storage.config_by_stage(stage)?;
    Ok(VestingRoundResponse {
        stage: Uint128::from(stage),
        total_claimed: config.total_claimed,
        merkle_root: config.merkle_root,
        token_address: deps.api.human_address(&config.token_address)?.to_string(),
        created_at: config.created_at,
    })
}

fn get_current_stage<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Uint128> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let current_stage = Uint128::from(config_storage.current_stage());
    Ok(current_stage)
}

fn get_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HumanAddr> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let contract_owner = config_storage.contract_owner()?;
    Ok(deps.api.human_address(&contract_owner)?)
}

fn get_granted_contract_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HumanAddr> {
    let config_storage = ReadonlyConfig::from_storage(&deps.storage);
    let contract_owner = config_storage.granted_contract_owner()?;
    Ok(deps.api.human_address(&contract_owner)?)
}
