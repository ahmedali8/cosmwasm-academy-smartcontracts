use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Define a variant called Value that takes no parameters.
    #[returns(ValueResp)]
    Value {},
}

#[cw_serde]
pub enum ExecMsg {
    // Define a variant called Donate that takes no parameters.
    Donate {},

    // Define a variant called Reset that takes a single parameter called counter which defaults to 0.
    Reset {
        #[serde(default)]
        counter: u64,
    },

    // Define a variant called Withdraw that takes no parameters.
    Withdraw {},

    WithdrawTo {
        receiver: String,
        #[serde(default)]
        funds: Vec<Coin>,
    },
}

#[cw_serde]
pub struct Parent {
    pub addr: String,
    pub donating_period: u64,
    pub part: Decimal,
}

#[cw_serde]
pub struct InstantiateMsg {
    // Define a field called counter of type u64 which defaults to 0.
    #[serde(default)]
    pub counter: u64,

    // Define a field called minimal_donation of type Coin.
    pub minimal_donation: Coin,

    pub parent: Option<Parent>,
}

#[cw_serde]
pub struct MigrateMsg {
    pub parent: Option<Parent>,
}

#[cw_serde]
pub struct ValueResp {
    // Define a field called value of type u64.
    pub value: u64,
}
