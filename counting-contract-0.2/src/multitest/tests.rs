use crate::msg::ValueResp;
use cosmwasm_std::{coin, coins, Addr};
use counting_contract_0_1::multitest::contract::CountingContract as CountingContract_0_1;
use cw_multi_test::App;

use crate::{
    error::ContractError,
    state::{State, STATE},
};

use super::contract::CountingContract;

const ATOM: &str = "atom";

#[test]
fn query_value() {
    let sender = Addr::unchecked("sender");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &sender,
        "Counting contract",
        10,
        coin(10, ATOM),
    )
    .unwrap();

    let resp = contract.query_value(&app).unwrap();

    assert_eq!(resp.value, 10);
}

#[test]
fn donate_without_funds() {
    let sender = Addr::unchecked("sender");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &sender,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    // execute donate
    contract.donate(&mut app, &sender, &[]).unwrap();

    let resp = contract.query_value(&app).unwrap();

    assert_eq!(resp.value, 0);
}

#[test]
fn donate_with_funds() {
    let sender = Addr::unchecked("sender");

    let mut app: App = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &sender, coins(10, ATOM))
            .unwrap()
    });

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &sender,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    // execute donate
    contract
        .donate(&mut app, &sender, &coins(10, ATOM))
        .unwrap();

    let resp = contract.query_value(&app).unwrap();

    assert_eq!(resp.value, 1);
    assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
    assert_eq!(
        app.wrap().query_all_balances(contract.addr()).unwrap(),
        coins(10, ATOM)
    );
}

#[test]
fn donate_expecting_no_funds() {
    let sender = Addr::unchecked("sender");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &sender,
        "Counting contract",
        None,
        coin(0, ATOM),
    )
    .unwrap();

    // execute donate
    contract.donate(&mut app, &sender, &[]).unwrap();

    let resp = contract.query_value(&app).unwrap();

    assert_eq!(resp.value, 1);
}

#[test]
fn reset() {
    let sender = Addr::unchecked("sender");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &sender,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    // execute reset
    contract.reset(&mut app, &sender, 10).unwrap();

    let resp = contract.query_value(&app).unwrap();

    assert_eq!(resp.value, 10);
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

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    // execute donate (sender)
    contract
        .donate(&mut app, &sender, &coins(10, ATOM))
        .unwrap();

    // execute donate (owner)
    contract.donate(&mut app, &sender, &[]).unwrap();

    // execute withdraw
    contract.withdraw(&mut app, &owner).unwrap();

    assert_eq!(
        app.wrap().query_all_balances(owner).unwrap(),
        coins(10, ATOM)
    );

    assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
    assert_eq!(
        app.wrap()
            .query_all_balances(contract.addr().clone())
            .unwrap(),
        vec![]
    );
}

#[test]
fn withdraw_to() {
    let owner = Addr::unchecked("owner");
    let sender = Addr::unchecked("sender");
    let receiver = Addr::unchecked("receiver");

    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &sender, coins(10, ATOM))
            .unwrap();
    });

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    // execute donate (sender)
    contract
        .donate(&mut app, &sender, &coins(10, ATOM))
        .unwrap();

    // execute withdraw_to (owner -> receiver)
    contract
        .withdraw_to(&mut app, &owner, &receiver, coins(5, ATOM))
        .unwrap();

    assert_eq!(app.wrap().query_all_balances(owner).unwrap(), vec![]);
    assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
    assert_eq!(
        app.wrap().query_all_balances(receiver).unwrap(),
        coins(5, ATOM)
    );
    assert_eq!(
        app.wrap()
            .query_all_balances(contract.addr().clone())
            .unwrap(),
        coins(5, ATOM)
    );
}

#[test]
fn unauthorized_withdraw() {
    let owner = Addr::unchecked("owner");
    let member = Addr::unchecked("member");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        None,
        coin(10, ATOM),
    )
    .unwrap();

    let err = contract.withdraw(&mut app, &member).unwrap_err();
    assert_eq!(
        err,
        ContractError::Unauthorized {
            owner: owner.into()
        }
    );
}

#[test]
fn unauthorized_withdraw_to() {
    let owner = Addr::unchecked("owner");
    let member = Addr::unchecked("member");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        None,
        coin(0, ATOM),
    )
    .unwrap();

    let err = contract
        .withdraw_to(&mut app, &member, &owner, None)
        .unwrap_err();

    assert_eq!(
        err,
        ContractError::Unauthorized {
            owner: owner.into()
        }
    );
}

#[test]
fn unauthorized_reset() {
    let owner = Addr::unchecked("owner");
    let member = Addr::unchecked("member");

    let mut app = App::default();

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        None,
        coin(0, ATOM),
    )
    .unwrap();

    let err = contract.reset(&mut app, &member, 10).unwrap_err();

    assert_eq!(
        err,
        ContractError::Unauthorized {
            owner: owner.into()
        }
    );
}

#[test]
fn migration() {
    let admin = Addr::unchecked("admin");
    let owner = Addr::unchecked("owner");
    let sender = Addr::unchecked("sender");

    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &sender, coins(10, ATOM))
            .unwrap()
    });

    let old_code_id = CountingContract_0_1::store_code(&mut app);
    let new_code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract_0_1::instantiate(
        &mut app,
        old_code_id,
        &owner,
        "Counting contract",
        &admin,
        None,
        coin(10, ATOM),
    )
    .unwrap();

    contract
        .donate(&mut app, &sender, &coins(10, ATOM))
        .unwrap();

    let contract =
        CountingContract::migrate(&mut app, contract.into(), new_code_id, &admin).unwrap();

    let resp = contract.query_value(&app).unwrap();
    assert_eq!(resp, ValueResp { value: 1 });

    let state = STATE.query(&app.wrap(), contract.addr().clone()).unwrap();
    assert_eq!(
        state,
        State {
            counter: 1,
            minimal_donation: coin(10, ATOM),
            owner
        }
    );
}

#[test]
fn migration_same_version() {
    let admin = Addr::unchecked("admin");
    let owner = Addr::unchecked("owner");
    let sender = Addr::unchecked("sender");

    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &sender, coins(10, ATOM))
            .unwrap()
    });

    let code_id = CountingContract::store_code(&mut app);

    let contract = CountingContract_0_1::instantiate(
        &mut app,
        code_id,
        &owner,
        "Counting contract",
        &admin,
        None,
        coin(10, ATOM),
    )
    .unwrap();

    contract
        .donate(&mut app, &sender, &coins(10, ATOM))
        .unwrap();

    let contract = CountingContract::migrate(&mut app, contract.into(), code_id, &admin).unwrap();

    let resp = contract.query_value(&app).unwrap();
    assert_eq!(resp, ValueResp { value: 1 });

    let state = STATE.query(&app.wrap(), contract.addr().clone()).unwrap();
    assert_eq!(
        state,
        State {
            counter: 1,
            minimal_donation: coin(10, ATOM),
            owner
        }
    );
}
