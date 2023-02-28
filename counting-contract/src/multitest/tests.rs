use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::App;

use super::contract::CountingContract;

const ATOM: &str = "atom";

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
