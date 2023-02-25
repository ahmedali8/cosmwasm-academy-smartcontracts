// Import various items from the `cosmwasm_std` library
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use msg::InstantiateMsg;

// Import the `contract` module, the `msg`, and the `state` module from the current crate
mod contract;
pub mod msg;
mod state;

// Define the `instantiate` entry point function, which is called when a new contract is deployed to the blockchain
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    contract::instantiate(deps, msg.counter)
}

// Define the `query` entry point function, which is called when a read-only operation is performed on the contract
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
    // Import the `query` function from the `contract` module and the `QueryMsg` enum variants from the `msg` module
    use contract::query;
    use msg::QueryMsg::*;

    // Match the input `msg` argument against the `QueryMsg` enum variants
    match msg {
        // If the input message is `Value`, call the `query::value(deps)?` function and serialize the result to a `Binary` value using the `to_binary` function
        Value {} => to_binary(&query::value(deps)?),
        // If the input message is `Incremented`, call the `query::incremented` function with the `value` parameter and serialize the result to a `Binary` value using the `to_binary` function
        Incremented { value } => to_binary(&query::incremented(value)),
    }
}
// Define the `execute` entry point function, which is called when a write operation is performed on the contract
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: msg::ExecMsg,
) -> StdResult<Response> {
    use contract::exec;
    use msg::ExecMsg::*;

    match msg {
        Poke {} => exec::poke(deps, info),
    }
}

// Define a test module for the contract
#[cfg(test)]
mod test {
    // Import various items from the current crate and from external libraries
    use crate::{
        execute, instantiate,
        msg::{ExecMsg, InstantiateMsg, QueryMsg, ValueResp},
        query,
    };
    use cosmwasm_std::{Addr, Empty};
    use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

    // Define a helper function that returns a boxed version of the contract for use in tests
    fn counting_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(execute, instantiate, query);
        Box::new(contract)
    }

    // Define a test function that checks that the `query::value()` function returns the expected result
    #[test]
    fn query_value() {
        // Create a new `App` instance, which represents the blockchain environment for testing
        let mut app: App = App::default();

        // Store the compiled contract code on the blockchain and get the resulting contract ID
        let contract_id: u64 = app.store_code(counting_contract());

        // Instantiate a new contract instance using the contract ID and a sender address, and get the resulting contract address
        let contract_addr: Addr = app
            .instantiate_contract(
                contract_id,
                Addr::unchecked("sender"),
                &InstantiateMsg { counter: 10 },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // Call the `query_wasm_smart` function on the contract instance with a `Value` query message, and deserialize
        // the resulting `Binary` value into a `ValueResp` struct
        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Value {})
            .unwrap();

        // Check that the response value matches the expected value of 10
        assert_eq!(resp, ValueResp { value: 10 });
    }

    // Define a test function that checks that the `query::incremented()` function returns the expected result
    #[test]
    fn query_incremented_value() {
        // Create a new `App` instance, which represents the blockchain environment for testing
        let mut app: App = App::default();

        // Store the compiled contract code on the blockchain and get the resulting contract ID
        let contract_id: u64 = app.store_code(counting_contract());

        // Instantiate a new contract instance using the contract ID and a sender address, and get the resulting contract address
        let contract_addr: Addr = app
            .instantiate_contract(
                contract_id,
                Addr::unchecked("sender"),
                &InstantiateMsg { counter: 0 },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // Query the `incremented` function with an input value of 3
        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Incremented { value: 3 })
            .unwrap();

        // Ensure that the response matches the expected result
        assert_eq!(resp, ValueResp { value: 4 });
    }

    #[test]
    fn poke() {
        let mut app = App::default();

        let sender = Addr::unchecked("sender");

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                sender.clone(),
                &InstantiateMsg { counter: 0 },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute poke
        let _poke_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Poke {},
                &[],
            )
            .unwrap();

        // println!("{:?}", poke_resp);

        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Value {})
            .unwrap();

        assert_eq!(resp, ValueResp { value: 1 });
    }
}
