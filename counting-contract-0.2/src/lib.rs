#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use error::ContractError;
use msg::InstantiateMsg;

// Import the `contract` module, the `msg`, and the `state` module from the current crate
mod contract;
pub mod error;
pub mod msg;
#[cfg(any(test, feature = "tests"))]
pub mod multitest;
mod state;

// Define the `instantiate` entry point function, which is called when a new contract is deployed to the blockchain
// This attribute is used to mark the function as an entry point for the smart contract.
// It is conditionally compiled with a feature flag to prevent it from being included in the library version of the code.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    contract::instantiate(deps, info, msg.counter, msg.minimal_donation)
}

// Define the `query` entry point function, which is called when a read-only operation is performed on the contract
// This attribute is used to mark the function as an entry point for the smart contract.
// It is conditionally compiled with a feature flag to prevent it from being included in the library version of the code.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
    // Import the `query` function from the `contract` module and the `QueryMsg` enum variants from the `msg` module
    use contract::query;
    use msg::QueryMsg::*;

    // Match the input `msg` argument against the `QueryMsg` enum variants
    match msg {
        // If the input message is `Value`, call the `query::value(deps)?` function and serialize the result to a `Binary` value using the `to_binary` function
        Value {} => to_binary(&query::value(deps)?),
    }
}

// Define the `execute` entry point function, which is called when a write operation is performed on the contract
// This attribute is used to mark the function as an entry point for the smart contract.
// It is conditionally compiled with a feature flag to prevent it from being included in the library version of the code.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: msg::ExecMsg,
) -> Result<Response, ContractError> {
    use contract::exec;
    use msg::ExecMsg::*;

    match msg {
        Donate {} => exec::donate(deps, info).map_err(ContractError::Std),
        Reset { counter } => exec::reset(deps, info, counter),
        Withdraw {} => exec::withdraw(deps, env, info),
        WithdrawTo { receiver, funds } => exec::withdraw_to(deps, env, info, receiver, funds),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    contract::migrate(deps)
}
