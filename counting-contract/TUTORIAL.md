# Execution entry point

Our smart contract has a state, so it is an excellent time to try to update it. In this lesson, you will implement the execution message to update the internal contract counter.

## Defining the message

The best place to start is to create a new message for the execute entry point. I suggest the name Poke here, as the message would poke the contract to increment its counter.

Let's open the `src/msg.rs` file and add a new enum there:

```rust
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  #[serde(rename_all = "snake_case")]
  pub enum ExecMsg {
      Poke {},
  }
```

As you can see, nothing new happens here - we define an enum with a single variant per execution message we want to handle.

## Creating the handler

The next step is to create a message handler. We would do it in the `src/contract.rs` in the dedicated submodule:

```rust
  pub mod exec {
      use cosmwasm_std::{DepsMut, Response, StdResult};

      use crate::state::COUNTER;

      pub fn poke(deps: DepsMut) -> StdResult<Response> {
          COUNTER.update(deps.storage, |counter| -> StdResult<_> { Ok(counter + 1) })?;

          Ok(Response::new())
      }
  }
```

The function is very similar to our `instantiate`, but instead of just storing value in the `COUNTER`, we are using the update function to [update](https://docs.rs/cw-storage-plus/0.14.0/cw_storage_plus/struct.Item.html#method.update) the underlying value.

The `update` function takes the borrow to the storage object and then the closure, which would be executed on the underlying object. The value returned by the closure should be a `Result` with the type stored as a `COUNTER` in an `Ok` variant. The Err variant can be anything implementing `From<cosmwasm_std::StdError>`.

Because the error type is never used here, and Rust has to know what type it should use, the type hint for the type returned from closure has to be provided - the `Ok` variant can be omitted, as the compiler can figure it out, but the error type has to be fixed here. The `StdResult` has its `Err` variant set, so it helps the compiler handle the situation.

Now let's dispatch on the new message in the entry point:

```rust
  #[entry_point]
  pub fn execute(
      deps: DepsMut,
      _env: Env,
      _info: MessageInfo,
      msg: msg::ExecMsg,
  ) -> StdResult<Response> {
      use contract::exec;
      use msg::ExecMsg::*;

      match msg {
          Poke {} => exec::poke(deps),
      }
  }
```

Very standard, just like our query message. The `execute` message is now ready, and it is a good point to check the contract's validity.

## Events and attributes

The execution is working, but to be honest - it is not yet complete. Execution is a good place to talk about events and attributes.

Every execution (and other action, like instantiation) emits events. Events are logs reporting what was perfromed by an action. Event contains a type and the set of key-value pairs (both key and value being strings) named attributes.

Events are emitted from execution using the `Response::add_event` function, passing the constructed `Event` type.

Every execution emits at least one default event, with the type of `wasm`. In most cases, it is good enough to emit only that one. To add attributes to the `wasm` event, we can use a `Response::add_attribute` function. That is what we would do in our contract:

```rust
  pub mod exec {
      use cosmwasm_std::{DepsMut, MessageInfo, Response, StdResult};

      use crate::state::COUNTER;

      pub fn poke(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
          let counter = COUNTER.load(deps.storage)? + 1;
          COUNTER.save(deps.storage, &counter)?;

          let resp = Response::new()
              .add_attribute("action", "poke")
              .add_attribute("sender", info.sender.as_str())
              .add_attribute("counter", counter.to_string());

          Ok(resp)
      }
  }
```

We changed a couple of things here. First, updating the counter is split - the purpose of that is to keep the new counter value for further usage. I always prefer updating a state using the `update` function if I don't need the old or updated value, but in a case like this, it is clearer to make it this way.

Additionally, I added a new argument to the execution function - the `MessageInfo`. It contains additional metadata about the sent message - the message sender and the funds sent. That is the proper way to detect the actual sender of the message - if it were added as a field on the message, it would be easily falsified, while the address in `MessageInfo` is verified by the blockchain and can be trusted. The funds are something we will talk about in the future. Now you can ignore this field.

Finally, before returning the `Response` object, we added three attributes to it - `action`, `sender`, and `counter`. `action` and `sender` are pretty much standard, and I encourage you to set it on every single execution your contract perform. The counter is very specific to the contract.

The last touch is to align the call of the `exec::poke` in the entry point:

```rust
  #[entry_point]
  pub fn execute(
      deps: DepsMut,
      _env: Env,
      info: MessageInfo,
      msg: msg::ExecMsg,
  ) -> StdResult<Response> {
      use contract::exec;
      use msg::ExecMsg::*;

      match msg {
          Poke {} => exec::poke(deps, info),
      }
  }
```

## Testing

As the new message handling is done, there is one more thing to do: we want to test it. We would use the multitest again:

```rust
  #[test]
  fn poke() {
      let mut app = App::default();

      let contract_id = app.store_code(counting_contract());

      let contract_addr = app
          .instantiate_contract(
              contract_id,
              Addr::unchecked("sender"),
              &InstantiateMsg { counter: 0 },
              &[],
              "Counting contract",
              None,
          )
          .unwrap();

      app.execute_contract(
          Addr::unchecked("sender"),
          contract_addr.clone(),
          &ExecMsg::Poke {},
          &[],
      )
      .unwrap();

      let resp: ValueResp = app
          .wrap()
          .query_wasm_smart(contract_addr, &QueryMsg::Value {})
          .unwrap();

      assert_eq!(resp, ValueResp { value: 1 });
  }
```

The test looks very similar to the `query_value` test. The new thing is the [App::execute_contract](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/trait.Executor.html#method.execute_contract) call, but it should not be very mysterious. Arguments it takes are the address sending the message, the contract address to receive it, the message itself, and the funds sent with it. We still ignore funds, setting them to the empty slice.

The thing that might surprise you might be the message passed to the `App::instantiate_contract` - it was an empty message before. That comes from a proposed solution to a previous lesson assignment. If it doesn't match your code, just leave what is a proper instantiation for your codebase right now.

## Assignment

Add another execution message, which would reset an internal counter (set it to given value).

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/cbf3fc344ca13d4c087b0320daf173e868b9bd63)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/8a6685d1d42328763f8b08563ba381adc8d4fc09)
