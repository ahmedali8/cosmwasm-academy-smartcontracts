use cosmwasm_std::{Addr, Coin, DepsMut, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

use crate::{
    error::ContractError,
    msg::Parent,
    state::{ParentDonation, State, PARENT_DONATION, STATE},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    info: MessageInfo,
    counter: u64,
    minimal_donation: Coin,
    parent: Option<Parent>,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    STATE.save(
        deps.storage,
        &State {
            counter,
            minimal_donation,
            owner: info.sender,
            donating_parent: parent.as_ref().map(|p| p.donating_period),
        },
    )?;

    if let Some(parent) = parent {
        PARENT_DONATION.save(
            deps.storage,
            &ParentDonation {
                address: deps.api.addr_validate(&parent.addr)?,
                donating_parent_period: parent.donating_period,
                part: parent.part,
            },
        )?;
    }

    // Return a new `Response` with no data or log messages
    Ok(Response::new())
}

pub fn migrate(mut deps: DepsMut) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    if contract_version.contract != CONTRACT_NAME {
        return Err(ContractError::InvalidContract {
            contract: contract_version.contract,
        });
    }

    let resp = match contract_version.version.as_str() {
        "0.1.0" => migrate_0_1_0(deps.branch()).map_err(ContractError::from)?,
        "0.2.0" => migrate_0_2_0(deps.branch()).map_err(ContractError::from)?,
        version => {
            if version == CONTRACT_VERSION {
                return Ok(Response::new());
            }

            return Err(ContractError::InvalidContractVersion {
                version: version.into(),
            });
        }
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(resp)
}

pub fn migrate_0_1_0(deps: DepsMut) -> StdResult<Response> {
    const COUNTER: Item<u64> = Item::new("counter");
    const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");
    const OWNER: Item<Addr> = Item::new("owner");

    let counter = COUNTER.load(deps.storage)?;
    let minimal_donation = MINIMAL_DONATION.load(deps.storage)?;
    let owner = OWNER.load(deps.storage)?;

    STATE.save(
        deps.storage,
        &State {
            counter,
            minimal_donation,
            owner,
            donating_parent: None,
        },
    )?;

    Ok(Response::new())
}

pub fn migrate_0_2_0(deps: DepsMut) -> StdResult<Response> {
    #[derive(Serialize, Deserialize)]
    struct OldState {
        pub counter: u64,
        pub minimal_donation: Coin,
        pub owner: Addr,
    }

    const OLD_STATE: Item<OldState> = Item::new("state");

    let OldState {
        counter,
        minimal_donation,
        owner,
    } = OLD_STATE.load(deps.storage)?;

    STATE.save(
        deps.storage,
        &State {
            counter,
            minimal_donation,
            owner,
            donating_parent: None,
        },
    )?;

    Ok(Response::new())
}

// Define a new module called `query`
pub mod query {
    use cosmwasm_std::{Deps, StdResult};

    // Import the `ValueResp` struct from the `msg` module
    use crate::{msg::ValueResp, state::STATE};

    // Define a public function called `value` that takes no arguments and returns a `ValueResp` struct
    pub fn value(deps: Deps) -> StdResult<ValueResp> {
        let value: u64 = STATE.load(deps.storage)?.counter;

        Ok(ValueResp { value })
    }
}

// Define a new module called `exec`
pub mod exec {
    use cosmwasm_std::{
        to_binary, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, WasmMsg,
    };

    use crate::{
        error::ContractError,
        msg::ExecMsg,
        state::{PARENT_DONATION, STATE},
    };

    pub fn donate(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
        let mut state = STATE.load(deps.storage)?;
        let mut resp = Response::new();

        if state.minimal_donation.amount.is_zero()
            || info.funds.iter().any(|coin| {
                coin.denom == state.minimal_donation.denom
                    && coin.amount >= state.minimal_donation.amount
            })
        {
            state.counter += 1;

            if let Some(parent) = &mut state.donating_parent {
                *parent -= 1;

                if *parent == 0 {
                    let parent_donation = PARENT_DONATION.load(deps.storage)?;

                    let funds: Vec<Coin> = deps
                        .querier
                        .query_all_balances(env.contract.address)?
                        .into_iter()
                        .map(|mut coin| {
                            coin.amount = coin.amount * parent_donation.part;
                            coin
                        })
                        .collect();

                    let msg = WasmMsg::Execute {
                        contract_addr: parent_donation.address.to_string(),
                        msg: to_binary(&ExecMsg::Donate {})?,
                        funds,
                    };

                    resp = resp
                        .add_message(msg)
                        .add_attribute("donated_to_parent", parent_donation.address.to_string());
                }
            }

            STATE.save(deps.storage, &state)?;
        }

        resp = resp
            .add_attribute("action", "donate")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", state.counter.to_string());

        Ok(resp)
    }

    pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        let owner = STATE.load(deps.storage)?.owner;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {
                owner: owner.to_string(),
            });
        }

        let balance = deps.querier.query_all_balances(&env.contract.address)?;

        // here msg.sender is this contract
        let bank_msg = BankMsg::Send {
            to_address: owner.to_string(),
            amount: balance,
        };

        let resp = Response::new()
            .add_message(bank_msg)
            .add_attribute("action", "withdraw")
            .add_attribute("sender", info.sender.as_str());

        Ok(resp)
    }

    pub fn withdraw_to(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        receiver: String,
        funds: Vec<Coin>,
    ) -> Result<Response, ContractError> {
        let owner = STATE.load(deps.storage)?.owner;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {
                owner: owner.to_string(),
            });
        }

        // Query the current balance of the contract's address from the blockchain
        let mut balance: Vec<Coin> = deps.querier.query_all_balances(&env.contract.address)?;

        // Check if there are any funds provided in the message info
        if !funds.is_empty() {
            // If funds were provided, iterate over each coin in the balance
            for coin in &mut balance {
                // Find the corresponding amount limit for the current coin from the provided funds (if any)
                let limit = funds
                    .iter()
                    .find(|c| c.denom == coin.denom)
                    .map(|c| c.amount)
                    .unwrap_or(Uint128::zero());

                // Set the coin amount to the minimum of the current amount and the limit (if there is a limit)
                coin.amount = std::cmp::min(coin.amount, limit);
            }
        }

        // here msg.sender is this contract
        let bank_msg = BankMsg::Send {
            to_address: receiver,
            amount: funds,
        };

        let resp = Response::new()
            .add_message(bank_msg)
            .add_attribute("action", "withdraw")
            .add_attribute("sender", info.sender.as_str());

        Ok(resp)
    }

    pub fn reset(
        deps: DepsMut,
        info: MessageInfo,
        counter: u64,
    ) -> Result<Response, ContractError> {
        let mut state = STATE.load(deps.storage)?;
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {
                owner: state.owner.to_string(),
            });
        }

        state.counter = counter;
        STATE.save(deps.storage, &state)?;

        let resp: Response = Response::new()
            .add_attribute("action", "reset")
            .add_attribute("sender", info.sender.as_str())
            .add_attribute("counter", counter.to_string());

        Ok(resp)
    }
}
