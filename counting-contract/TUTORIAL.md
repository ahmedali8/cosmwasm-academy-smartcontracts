# Errors handling

Our `counting-contract` reports an error in case an unauthorized address calls some executions. Unfortunately - error handling is very poor, string based. It also makes testing errors a bit of a problem. Let's improve on that.

## The error type

To define our errors, we would use the [thiserror](https://docs.rs/thiserror/1.0.33/thiserror/) crate. We need to add it to our project:

```bash
  $ cargo add thiserror
```

Then we want to create a custom error type for our contract. We will put it in a new module in `src/error.rs` file - don't forget to add a module in your `src/lib.rs`:

```rust
  use cosmwasm_std::StdError;
  use thiserror::Error;

  #[derive(Error, Debug, PartialEq)]
  pub enum ContractError {
      #[error("{0}")]
      Std(#[from] StdError),

      #[error("Unauthorized - only {owner} can call it")]
      Unauthorized { owner: String },
  }
```

Using `thiserror`, we define our error types as simple enum types. Deriving the [thiserror::Error](https://docs.rs/thiserror/1.0.33/thiserror/derive.Error.html) trait generates all the boilerplate, so the error is implementing `std::error::Error` trait. Things we need to deliver for it to work are the implementation of `Debug` - which is often derived, and the information on how the error should be converted to a string. This is achieved by putting an `#[error(...)]` attribute on all enum variants containing the format string for this variant.

For our contract, the most important error is and `Unauthorized` variant - it is the only thing we return from the contract manually. But it is also crucial to implement the other `Std` variant. It wraps any error of the `StdError` type, which could be returned by CosmWasm standard library or utilities. This way, we can use the `ContractError` type in our smart contract, still being able to return errors occurring in `cosmwasm-std`. The additional `#[from]` attribute tells thiserror to generate the `From` trait, converting the underlying type to the error variant (in this case: `impl From<StdError> for ContractError`). This enables using the `?` operator forwarding `StdError` in functions returning `ContractError`.

## Returning an error

Now having an error, we can use it in the smart contract. Let's update the `withdraw` function:

```rust
  pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
      let owner = OWNER.load(deps.storage)?;
      if info.sender != owner {
          return Err(ContractError::Unauthorized {
              owner: owner.to_string(),
          });
      }

      let balance = deps.querier.query_all_balances(&env.contract.address)?;
      let bank_msg = BankMsg::Send {
          to_address: info.sender.to_string(),
          amount: balance,
      };

      let resp = Response::new()
          .add_message(bank_msg)
          .add_attribute("action", "withdraw")
          .add_attribute("sender", info.sender.as_str());

      Ok(resp)
  }
```

The change here is simple - instead of returning `StdError::generic_error(...)`, we return our custom build `ContractError::Unauthorized`. The great upside of this approach is that as there are no more magic strings in error reporting, it is more difficult to make a typo. Before, we could easily mismatch `Unauthorized` spelling in one place and be left with inconsistent error reporting. Now, the error is structured strongly by the type system, and the typo can happen in a single place - and if it happens, it is easier to correct consistently along all usages.

You also need to remember to update the return type of the withdraw method - we don't want to return `StdResult` anymore. Instead, we switch to `Result<Response, ContractError>`. We need to make this change in the entry point too:

```rust
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
          Reset { counter } => exec::reset(deps, info, counter).map_err(ContractError::Std),
          Withdraw {} => exec::withdraw(deps, env, info),
          WithdrawTo { receiver, funds } => {
              exec::withdraw_to(deps, env, info, receiver, funds).map_err(ContractError::Std)
          }
      }
  }
```

We had to make another alignment in the entry point. Some of our handlers still return a `StdError` type on the error case. It is ok for us - we have a variant in our `ContractError` for it, so we just map the error case to the proper value - we are using the enum variant as a single argument function constructing the value. We could also use any of `From::from`, `Into::into, ContractError::from`, probably others I don't remember - I like using the way I showed as I find it very clear and expressive.

Now is a good time to ensure the regression and contract checks are passing.

## Testing

I mentioned before that returning errors via string doesn't make them very testable. The reason is to test them reasonably. You would have to compare returned error string, which looks bad, especially for longer error descriptions. It is also prone to any changes in errors - tests would start to fail one by one if you would change the format of your error.

To make testing easier, multitest uses the [anyhow](https://docs.rs/anyhow/1.0.63/anyhow/) crate, authored by the same person as the `thiserror` crate. It allows it to forward all occurred errors in a type-erased way, so they can be later reconstructed to verify their structure. Let's see how it works in real live testing for an error path.

```rust
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
                  minimal_donation: coin(10, "atom"),
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
```

The part of the test until the `Withdraw` execution is standard and has already been discussed. We don't need any funds sent here, as we just want to call `Withdraw` by an unauthorized address. As we expect the operation to fail, instead of calling `unwrap` on the result, we use [unwrap_err](https://doc.rust-lang.org/std/result/enum.Result.html#method.unwrap_err), working symmetrically but expecting `Result` to contain an `Err` variant. Then we can compare and error to an expected value, using the magic of `anyhow`. To make it work, we need to keep an expected error as the first side of `assert_eq`. We can call a [downcast](https://docs.rs/anyhow/1.0.63/anyhow/struct.Error.html#method.downcast) function on returned error, which tries to convert a type-erased error type to what we expect. The operation would fail if the error was created from the other type we are trying to convert it to - therefore, we have to unwrap it. The Rust compiler elides the type we expect to be returned to be the same type as the first argument of `assert_eq` - that is why we wanted it to be the first one. If you prefer to keep the expected value as the second part of `partial_eq`, there is a way around it - you can use the turbofish syntax (downcasting by `err.downcast::<ContractError>().unwrap()`).

## Assignment

Make sure, that no function reports a failure via `StdError::generic_err(...)` - it should use `ContractError` instead. Then write at least one error path test scenario for all your functions which may fail.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/21a9778396088526b72d2d8e7552016baf4c2ba3)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/9560de745cc0629d8797dbf88ddbe5c97af0dcc7)
