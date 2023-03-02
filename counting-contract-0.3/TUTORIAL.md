# Calling external contracts

In our journey, we implemented donations counting smart contract. We learn a solid foundation to create single contracts on the chain, but in real life, we would often have systems using multiple smart contracts. We will not dive into building such systems, but I want to show you the basics of how to trigger the execution of external contracts in CosmWasm.

## Preparing a state

We are adding new functionality, so let's start copying the contract and creating new version (remember to update it in `Cargo.toml`:

```bash
  $ cp -r ./counting-contract-0.2/ ./counting-contract-0.3
```

We want to add a functionality to our contract, so every couple of donations sends part of accumulated funds to another "parent" counting contract using the `Donate` message. We would allow configuring both frequency and part of funds to donate.

Let's start adding some more state to the contract:

```rust
  use cosmwasm_std::Decimal;

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
  pub struct State {
      pub counter: u64,
      pub minimal_donation: Coin,
      pub owner: Addr,
      pub donating_parent: Option<u64>,
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  pub struct ParentDonation {
      pub address: Addr,
      pub donating_parent_period: u64,
      pub part: Decimal,
  }

  pub const STATE: Item<State> = Item::new("state");
  pub const PARENT_DONATION: Item<ParentDonation> = Item::new("parent_donation");
```

We added two things to state. The `State` structure got enriched with `donation_parent` field. It would be a countown till the donation forwarding is supposed to happen. The `donating_parent_period` would be a value it should be reset to when reaches `0`. Now we need some more things in the instantiation message:

```rust
  use cosmwasm_std::Decimal;

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
  pub struct Parent {
      pub addr: String,
      pub donating_period: u64,
      pub part: Decimal,
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub struct InstantiateMsg {
      #[serde(default)]
      pub counter: u64,
      pub minimal_donation: Coin,
      pub parent: Option<Parent>,
  }
```

We added a new embedded structure to keep the information about forwarding to the parent contract. If this is not set, we will disable this functionality. Obviously, we also need to update the instantiation function (then remember to update our entry point with the new argument):

```rust
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

      Ok(Response::new())
  }
```

Finally, we also need to update the migration. We need to update the `migrate_0_1_0` as well to add a function to migrate from "0.2" version. To keep things simple, we will assume that the old contract didn't use the parent functionality, so he still doesn't need it:

```rust
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
          CONTRACT_VERSION => return Ok(Response::default()),
          version => {
              return Err(ContractError::InvalidContractVersion {
                  version: version.into(),
              })
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
          counter: u64,
          minimal_donation: Coin,
          owner: Addr,
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
```

In the migration from "0.2" you may find a nice use of destructurization in rust. Also, it is worth noting that it is not needed to have this second migration - it turns out that the old and new State are serializing to the same JSON if there is no parent because it is optional. However, it is no harm to having them - this way, we are sure we do not mess something up.

## Forwarding the donation

Now we need to add the new functionality to the `donate` execution. Take a look at the code:

```rust
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
                  *parent = parent_donation.donating_parent_period;

                  let funds: Vec<_> = deps
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
          .add_attribute("action", "poke")
          .add_attribute("sender", info.sender.as_str())
          .add_attribute("counter", state.counter.to_string());

      Ok(resp)
  }
```

The code should be familiar - we use mostly knowledge we already have. Note that I moved the response creation to the very beginning of the function because I want to add sub-messages to it in the function implementation. The other way would be to store messages somewhere on the side and then add them to created response - we use both approaches commonly, depending on what is more convenient for a particular case.

To send a message to another contract, we do a similar thing to sending the funds - we use the `add_message` to schedule execution when this handler finishes its work. This time, instead of `BankMsg`, we send a [WasmMessage](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/enum.WasmMsg.html), which is designed to send contract-related messages. Here we are using the ExecMsg, but it can also be used to instantiate or migrate another contract.

You may be curious how the `add_message` handles accepting different message types, and the answer is simple: it uses a similar trick we used before to pass the option transparently. Instead of taking the concrete type, `add_message` accepts any argument implementing the [Into<CosmosMsg> trait](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/enum.CosmosMsg.html) - both `WasmMsg` and `BankMsg` fulfil that bound.

Finally, don't forget to update your entry point and tests, as our changes require some alignment there.

## Testing

Traditionally - new functionality requires testing. Let's see how multitest handles having multiple contracts created. In the end, it was the primary reason it was created (and that is why it has "multi" in its name!). Here is the test of the new functionality:

```rust
  #[test]
  fn donating_parent() {
      let owner = Addr::unchecked("owner");
      let sender = Addr::unchecked("sender");

      let mut app = App::new(|router, _api, storage| {
          router
              .bank
              .init_balance(storage, &sender, coins(20, "atom"))
              .unwrap();
      });

      let code_id = CountingContract::store_code(&mut app);

      let parent_contract = CountingContract::instantiate(
          &mut app,
          code_id,
          &owner,
          "Parent contract",
          None,
          None,
          coin(0, ATOM),
          None,
      )
      .unwrap();

      let contract = CountingContract::instantiate(
          &mut app,
          code_id,
          &owner,
          "Counting contract",
          None,
          None,
          coin(10, ATOM),
          Parent {
              addr: parent_contract.addr().to_string(),
              donating_period: 2,
              part: Decimal::percent(10),
          },
      )
      .unwrap();

      contract
          .donate(&mut app, &sender, &coins(10, ATOM))
          .unwrap();
      contract
          .donate(&mut app, &sender, &coins(10, ATOM))
          .unwrap();

      let resp = parent_contract.query_value(&app).unwrap();
      assert_eq!(resp, ValueResp { value: 1 });

      let resp = contract.query_value(&app).unwrap();
      assert_eq!(resp, ValueResp { value: 2 });

      assert_eq!(app.wrap().query_all_balances(owner).unwrap(), vec![]);
      assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
      assert_eq!(
          app.wrap().query_all_balances(contract.addr()).unwrap(),
          coins(18, ATOM)
      );
      assert_eq!(
          app.wrap()
              .query_all_balances(parent_contract.addr())
              .unwrap(),
          coins(2, ATOM)
      );
  }
```

Everything is natural, and following knowledge, we already have. We configured an environment with two counting contracts, one set as the parent of the other. To simplify the test, we set donating period to two donations and performed them. Then we verified if both contract counters have proper value and if all funds flow as we expected.

## Assignment

Update the migration so it allows to add the parent contract when migrating. You will need to add a new migration message for that.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/cb9e02e07d12fb5c8c2e90b444eb263a71b1cea5)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/4fd25ca81f5fe8e232f106b0a5b991188f027f1f)
