use fadroma::prelude::*;
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

#[message] pub struct InitMsg {}

/// Transactions that this contract supports.
#[message] pub enum HandleMsg {
    Echo,
    Fail
}

/// Queries that this contract supports.
#[message] pub enum QueryMsg {
    Schema
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>, _env: Env, msg: InitMsg,
) -> StdResult<InitResponse> {
    // TODO: save own address
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>, _env: Env, msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Echo => HandleResponse::default().data(&msg),
        HandleMsg::Fail => Err(StdError::generic_err("this transaction always fails"))
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>, msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Schema => {
            to_binary(&vec![
                &schema_for!(InitMsg),
                &schema_for!(HandleMsg),
                &schema_for!(QueryMsg),
            ])
        },
    }
}

fadroma::entrypoint!(fadroma, init, handle, query);