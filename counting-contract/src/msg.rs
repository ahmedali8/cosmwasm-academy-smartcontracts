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

// Define a struct called ValueResp that can be serialized and deserialized,
// can be cloned, can be debug printed, and can be compared for equality.
// The serde attribute renames the fields to snake_case during serialization.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ValueResp {
    // Define a field called value of type u64.
    pub value: u64,
}
