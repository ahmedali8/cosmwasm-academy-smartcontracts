use cosmwasm_std::{
    entry_point, Deps, DepsMut, Empty, Env, MessageInfo, QueryResponse, Response, StdError,
    StdResult,
};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<QueryResponse, StdError> {
    Ok(QueryResponse::default())
}
