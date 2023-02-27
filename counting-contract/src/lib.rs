// Import various items from the `cosmwasm_std` library
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use error::ContractError;
use msg::InstantiateMsg;

// Import the `contract` module, the `msg`, and the `state` module from the current crate
mod contract;
pub mod error;
pub mod msg;
mod state;

// Define the `instantiate` entry point function, which is called when a new contract is deployed to the blockchain
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    contract::instantiate(deps, info, msg.counter, msg.minimal_donation)
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

// Define a test module for the contract
#[cfg(test)]
mod test {
    use std::vec;

    // Import various items from the current crate and from external libraries
    use crate::{
        error::ContractError,
        execute, instantiate,
        msg::{ExecMsg, InstantiateMsg, QueryMsg, ValueResp},
        query,
    };
    use cosmwasm_std::{coin, coins, Addr, Empty};
    use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

    const ATOM: &str = "atom";

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
                &InstantiateMsg {
                    counter: 10,
                    minimal_donation: coin(10, ATOM),
                },
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
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
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
    fn donate() {
        let mut app = App::default();

        let sender = Addr::unchecked("sender");

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                sender.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute donate
        let _donate_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Donate {},
                &[],
            )
            .unwrap();

        // println!("{:?}", donate_resp);

        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Value {})
            .unwrap();

        assert_eq!(resp, ValueResp { value: 0 });
    }

    #[test]
    fn donate_with_funds() {
        let sender = Addr::unchecked("sender");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &sender, coins(10, ATOM))
                .unwrap();
        });

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                sender.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute donate
        let _donate_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Donate {},
                &coins(10, ATOM),
            )
            .unwrap();

        // println!("{:?}", donate_resp);

        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Value {})
            .unwrap();

        assert_eq!(resp, ValueResp { value: 1 });

        assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
        assert_eq!(
            app.wrap().query_all_balances(contract_addr).unwrap(),
            coins(10, ATOM)
        );
    }

    #[test]
    fn donate_expecting_no_funds() {
        let sender = Addr::unchecked("sender");

        let mut app = App::default();

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                sender.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(0, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute donate
        let _donate_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Donate {},
                &[],
            )
            .unwrap();

        // println!("{:?}", donate_resp);

        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr.clone(), &QueryMsg::Value {})
            .unwrap();

        assert_eq!(resp, ValueResp { value: 1 });
    }

    #[test]
    fn reset() {
        let mut app = App::default();

        let sender = Addr::unchecked("sender");

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                sender.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute reset
        let _reset_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Reset { counter: 10 },
                &[],
            )
            .unwrap();

        // println!("{:?}", reset_resp);

        let resp: ValueResp = app
            .wrap()
            .query_wasm_smart(contract_addr, &QueryMsg::Value {})
            .unwrap();

        assert_eq!(resp, ValueResp { value: 10 });
    }

    #[test]
    fn withdraw() {
        let sender = Addr::unchecked("sender");
        let owner = Addr::unchecked("owner");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &sender, coins(10, ATOM))
                .unwrap();
        });

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        // execute donate (sender)
        let _donate_resp: AppResponse = app
            .execute_contract(
                sender.clone(),
                contract_addr.clone(),
                &ExecMsg::Donate {},
                &coins(10, ATOM),
            )
            .unwrap();

        // println!("{:?}", donate_resp);

        // execute withdraw (owner)
        let _withdraw_resp: AppResponse = app
            .execute_contract(
                owner.clone(),
                contract_addr.clone(),
                &ExecMsg::Withdraw {},
                &[],
            )
            .unwrap();

        // println!("{:?}", withdraw_resp);

        assert_eq!(
            app.wrap().query_all_balances(owner).unwrap(),
            coins(10, ATOM)
        );
        assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
        assert_eq!(
            app.wrap().query_all_balances(contract_addr).unwrap(),
            vec![]
        );
    }

    #[test]
    pub fn withdraw_to() {
        const ATOM: &str = "atom";

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked("sender");
        let receiver = Addr::unchecked("receiver");

        let mut app = App::new(|router, _api, storage| {
            router
                .bank
                .init_balance(storage, &sender, coins(10, ATOM))
                .unwrap();
        });

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        app.execute_contract(
            sender.clone(),
            contract_addr.clone(),
            &ExecMsg::Donate {},
            &coins(10, ATOM),
        )
        .unwrap();

        app.execute_contract(
            owner.clone(),
            contract_addr.clone(),
            &ExecMsg::WithdrawTo {
                receiver: receiver.to_string(),
                funds: coins(5, ATOM),
            },
            &[],
        )
        .unwrap();

        assert_eq!(app.wrap().query_all_balances(owner).unwrap(), vec![]);
        assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
        assert_eq!(
            app.wrap().query_all_balances(receiver).unwrap(),
            coins(5, ATOM)
        );
        assert_eq!(
            app.wrap().query_all_balances(contract_addr).unwrap(),
            coins(5, ATOM)
        );
    }

    #[test]
    fn unauthorized_withdraw() {
        let owner = Addr::unchecked("owner");
        let member = Addr::unchecked("member");

        let mut app = App::default();

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(10, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(member, contract_addr, &ExecMsg::Withdraw {}, &[])
            .unwrap_err();

        assert_eq!(
            ContractError::Unauthorized {
                owner: owner.into()
            },
            err.downcast().unwrap()
        );
    }

    #[test]
    fn unauthorized_withdraw_to() {
        let owner = Addr::unchecked("owner");
        let member = Addr::unchecked("member");

        let mut app = App::default();

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(0, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                member,
                contract_addr,
                &ExecMsg::WithdrawTo {
                    receiver: owner.to_string(),
                    funds: vec![],
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(
            ContractError::Unauthorized {
                owner: owner.into()
            },
            err.downcast().unwrap()
        );
    }

    #[test]
    fn unauthorized_reset() {
        let owner = Addr::unchecked("owner");
        let member = Addr::unchecked("member");

        let mut app = App::default();

        let contract_id = app.store_code(counting_contract());

        let contract_addr = app
            .instantiate_contract(
                contract_id,
                owner.clone(),
                &InstantiateMsg {
                    counter: 0,
                    minimal_donation: coin(0, ATOM),
                },
                &[],
                "Counting contract",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(member, contract_addr, &ExecMsg::Reset { counter: 10 }, &[])
            .unwrap_err();

        assert_eq!(
            ContractError::Unauthorized {
                owner: owner.into()
            },
            err.downcast().unwrap()
        );
    }
}
