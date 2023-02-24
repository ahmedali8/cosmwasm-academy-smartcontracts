// Define a new module called `query`
pub mod query {
    use cosmwasm_std::{Deps, StdResult};

    // Import the `ValueResp` struct from the `msg` module
    use crate::{msg::ValueResp, state::COUNTER};

    // Define a public function called `value` that takes no arguments and returns a `ValueResp` struct
    pub fn value(deps: Deps) -> StdResult<ValueResp> {
        // Load the current value of the COUNTER item from storage and assign it to the value variable
        // The load() method takes a reference to the storage and returns a Result containing the loaded value if successful, or an error if not
        let value: u64 = COUNTER.load(deps.storage)?;

        Ok(ValueResp { value })
    }

    // Define a public function called `incremented` that takes a single `u64` argument and returns a `ValueResp` struct
    pub fn incremented(value: u64) -> ValueResp {
        // Create a new `ValueResp` struct with a `value` field set to the input `value` incremented by 1, and return it
        ValueResp { value: value + 1 }
    }
}
