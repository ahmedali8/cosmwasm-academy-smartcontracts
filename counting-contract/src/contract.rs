use cosmwasm_std::{Coin, DepsMut, Response, StdResult};

use crate::state::{COUNTER, MINIMAL_DONATION};

pub fn instantiate(deps: DepsMut, counter: u64, minimal_donation: Coin) -> StdResult<Response> {
    // Save the initial value of counter and minimal_donation to the storage.
    COUNTER.save(deps.storage, &counter)?;
    MINIMAL_DONATION.save(deps.storage, &minimal_donation)?;

    // Return a new `Response` with no data or log messages
    Ok(Response::new())
}

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

// Define a new module called `exec`
pub mod exec {
    use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult};

    use crate::state::{COUNTER, MINIMAL_DONATION};

    pub fn donate(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
        // COUNTER.update(deps.storage, |counter| -> StdResult<_> { Ok(counter + 1) })?;

        let mut counter: u64 = COUNTER.load(deps.storage)?;
        let minimal_donation = MINIMAL_DONATION.load(deps.storage)?;

        if minimal_donation.amount.is_zero()
            || info.funds.iter().any(|coin| {
                coin.denom == minimal_donation.denom && coin.amount >= minimal_donation.amount
            })
        {
            counter += 1;
            COUNTER.save(deps.storage, &counter)?;
        }

        let resp: Response = Response::new()
            .add_attribute("action", "donate")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", counter.to_string());

        Ok(resp)
    }

    pub fn reset(deps: DepsMut, info: MessageInfo, counter: u64) -> StdResult<Response> {
        COUNTER.save(deps.storage, &counter)?;

        let resp: Response = Response::new()
            .add_attribute("action", "reset")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", counter.to_string());

        Ok(resp)
    }
}
