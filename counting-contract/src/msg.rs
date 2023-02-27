use cosmwasm_std::Coin;
use serde::{Deserialize, Serialize};

// Define an enum called QueryMsg that can be serialized and deserialized,
// can be cloned, can be debug printed, and can be compared for equality.
// The serde attribute renames the variants to snake_case during serialization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Define a variant called Value that takes no parameters.
    Value {},
    // Define a variant called Incremented that takes a single parameter called value.
    Incremented { value: u64 },
}

// Define an enum called ExecMsg that can be serialized and deserialized,
// can be cloned, can be debug printed, and can be compared for equality.
// The serde attribute renames the variants to snake_case during serialization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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
}

// Define a struct called InstantiateMsg that can be serialized and deserialized,
// can be cloned, can be debug printed, and can be compared for equality.
// The serde attribute renames the fields to snake_case during serialization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    // Define a field called counter of type u64 which defaults to 0.
    #[serde(default)]
    pub counter: u64,

    // Define a field called minimal_donation of type Coin.
    pub minimal_donation: Coin,
}

// Define a struct called ValueResp that can be serialized and deserialized,
// can be cloned, can be debug printed, and can be compared for equality.
// The serde attribute renames the fields to snake_case during serialization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ValueResp {
    // Define a field called value of type u64.
    pub value: u64,
}
