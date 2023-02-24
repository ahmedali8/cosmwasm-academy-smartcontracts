// Import various items from the `cosmwasm_std` library
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

// Import the `contract` module and the `msg` module from the current crate
mod contract;
pub mod msg;

// Define the `instantiate` entry point function, which is called when a new contract is deployed to the blockchain
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    // Return a default `Response` with no data or log messages
    Ok(Response::default())
}

// Define the `query` entry point function, which is called when a read-only operation is performed on the contract
#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
    // Import the `query` function from the `contract` module and the `QueryMsg` enum variants from the `msg` module
    use contract::query;
    use msg::QueryMsg::*;

    // Match the input `msg` argument against the `QueryMsg` enum variants
    match msg {
        // If the input message is `Value`, call the `query::value()` function and serialize the result to a `Binary` value using the `to_binary` function
        Value {} => to_binary(&query::value()),
        // If the input message is `Incremented`, call the `query::incremented` function with the `value` parameter and serialize the result to a `Binary` value using the `to_binary` function
        Incremented { value } => to_binary(&query::incremented(value)),
    }
}
