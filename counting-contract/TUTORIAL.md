# Contracts testing

After the last lesson, you should already have a contract you can talk to - it is a good point to start testing it.

Another dependency would be helpful with that:

```bash
  $ cargo add --dev cw-multi-test
```

The [multitest](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/) crate is a framework we deliver to make it easy to create smart contract tests with a simulated real blockchain environment. The idea is that instead of testing contracts function by function, they are tested as black-boxed applications, just like they were uploaded to the blockchain. Then instead of testing the internal functions, tests send real JSON messages to its entry points. It would be possible to test smart contracts by setting up the local containerized testnet, and automatically uploading them to operate on them later, but this is a bit more work to do, and such tests tend to take more time to execute. They have important value, but we prefer to have most of the coverage covered in those simulated tests using our framework.

## Creating contract wrapper

The first thing you need to test your contract with the multitest is the contract wrapper which would forward all messages to the proper entry point. We would write a function that creates such a wrapper. As the contract is small, I will put tests directly in lib.rs file in the dedicated `tests` module: #[cfg(test)]

```rust
  mod test {
      use cosmwasm_std::Empty;
      use cw_multi_test::Contract;

      fn counting_contract() -> Box<dyn Contract<Empty>> {
          todo!()
      }
  }
```

There are a couple of things to explain here. Starting on top - the `#[cfg(test)]` attribute on the `tests` module. [cfg](https://doc.rust-lang.org/reference/conditional-compilation.html#the-cfg-attribute) is a conditional compilation attribute, meaning that the code it wraps would be compiled-in if and only if the predicate passed to it is true. In this case, we have `test` predicate, which is true on `cargo test` runs. Thanks to this, our test would not be unnecessarily sitting in the final binary where they are not needed.

The next thing is the [Contract](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/trait.Contract.html) trait from the multitest crate. It is a trait that defines the smart contract implementation for multitest. It is possible to implement your own structure and forward it to entry points, but we will utilize the [ContractWrapper](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.ContractWrapper.html) type to make it easier. The generic argument for the `Contract` trait is used to test with some blockchain-specific features. In this course, we are talking only about generic Cosm Wasm contracts, so this would always be `Empty` for us.

A small thing our contract still miss, and we need it for multitest, is the `execute` entry point - unless you added it yourself in your second lesson assignment. You need to add an execute entry point, very similar to the instantiate one:

```rust
  #[entry_point]
  pub fn execute(\_deps: DepsMut, \_env: Env, \_info: MessageInfo, \_msg: Empty) -> StdResult<Response> {
      Ok(Response::new())
  }
```

Now ypu are ready to add an implementation of `counting_contract` function

```rust
  use cw_multi_test::ContractWrapper;
  use crate::{execute, instantiate, query};

  fn counting_contract() -> Box<dyn Contract<Empty>> {
      let contract = ContractWrapper::new(execute, instantiate, query);
      Box::new(contract)
  }
```

It is straightforward - create a `ContractWrapper` instance passing all three basic entry points to it, and then return it boxed.

## Creating a test

Now it's time to add a test for the query

```rust
  #[test]
  fn query_value() {
      let mut app = App::default();

      let contract_id = app.store_code(counting_contract());

      let contract_addr = app
          .instantiate_contract(
              contract_id,
              Addr::unchecked("sender"),
              &Empty {},
              &[],
              "Counting contract",
              None,
          )
          .unwrap();

      let resp: ValueResp = app
          .wrap()
          .query_wasm_smart(contract_addr, &QueryMsg::Value {})
          .unwrap();

      assert_eq!(resp, ValueResp { value: 0 });
  }
```

### The _app_ object

Many things are happening here, so let me explain them one by one.

You should be familiar with the `#[test]` attribute - it tells cargo that the function is a unit test.

Then the first thing in this test is creating a default [App](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.App.html) instance. `App`, is the soul of the multitest framework - it is the blockchain simulator, and it would be an interface to all contracts on it.

### Storing the contract on the blockchain

We use it right after it is created to call [store_code](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.App.html#method.store_code) on it. There is no code stored anywhere, but it performs an equivalent of storing code on the blockchain, so this is the function's name - to make test matching operations on the blockchain as closely as possible.

Why do we have to store contracts in the blockchain? I like to visualize blockchain as an extensive database, particularly key-value storage. Smart contracts are some special values stored there. Every smart contract has its keys it is allowed to manage in this database, but also it has some WASM code that defines how it works. But instead of uploading the code for every single, smart contract, we made it possible to have multiple smart contracts using the same implementation. To achieve that, before creating (or more precisely: instantiating) a smart contract, we have to upload the code to the blockchain storage and then pass the id of this code to created contract. Storing code is this operation of uploading smart contract binary to be stored in a blockchain state.

### Contract instantiation

The next step is contract instantiation - creating the contract on the blockchain. The [instantiate_contract](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/trait.Executor.html#method.instantiate_contract) method requires some input.

The first is the "uploaded" code id - the one you got back from your `store_code` call. Then you need to pass the address which sends the instantiation message. Most calls in CosmWasms can use this information, and it is verified by blockchain. In the test, we pass the sender with the message. To create a CosmWasm address, we are using the [Addr::unchecked](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Addr.html#method.unchecked) function. It creates an address without validating it - it is not a good idea to use it in blockchain implementation. Still, it is very common to use it in tests, as here, we can use arbitrary strings for our addresses, so they are more readable in case of failures.

Then we pass the instantiation message, which would be sent to the contract. I am using the `Empty` message, but what is important - is it would not be just passed to the entry point. It would be first serialized to JSON and then deserialized back to send it to the contract. It may seem unnecessary work, but it allows for testing if APIs are correct - for example, if two different messages we assume serialize the same way are interchangeable.

After the message, there is a definition of native funds we want to send with the message. In CosmWasm, most messages can have some tokens sent with them, and they would be transferred to the destination contract if the message succeeds in executing. We will talk about dealing with those funds later, but until then, we pass an empty slice.

Funds are followed with the label of the contract. There is not too much to tell about it - it is just the human-readable name of the created contract.

Last is the admin of the contract. Admins are the only addresses that can later perform migrations of smart contracts. Similarly to funds - we do not care about them for now, so we just pass `None` to this, which means there is no admin, and no migrations would be allowed.

Note, we need to unwrap the result of instantiation - it is because the instantiation could fail - it returns the `Result` in the contract. Our particular contract never fails, but multitest doesn't know about this. Also, unwrapping in tests is nothing bad - if unwrapping fails, the test fails.

### Querying the contract

Finally, it's time to query the contract on the blockchain. To do so, we first need to call the [wrap()](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.App.html#method.wrap) method on the `app`. It converts the app object to a temporary [QuerierWrapper](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.QuerierWrapper.html) object, allowing us to query the blockchain. To query the contract, we use the [query_wasm_smart](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.QuerierWrapper.html#method.query_wasm_smart) function. Queries are simpler functions than instantiate. Depending on who calls them or receives any funds, they cannot modify the blockchain state. Because of that, the query needs only the queried contract address (which you got from instantiation) and the query message to send.

However, there is one more thing here - note that you need to put the type hint for the message's response. The multitest framework works with JSON messages and has no idea what to deserialize your response to unless you provide it with a hint. We can trick the Rust type elision by swapping the [assert_eq](https://doc.rust-lang.org/std/cmp/trait.Eq.html?search=assert_eq) arguments order, but I find this more consistent.

The last step is to ensure that the queried value is what we expect it to be - we use `assert_eq` here, which panics if its arguments are not equal.

## Running the test

Now it is time to run the new test using `cargo test`:

```bash
  $ cargo test
```

If it passes, it is also a good habit to always check if it is still a valid CosmWasm contract building it with cargo wasm, and validating with cosmwasm-check:

```bash
  $ cargo wasm
```

Now you have your smart contract query tested - it is time to add some state to it, so the value returned by a query is not a magic number.

## Assignment

Write the test for incrementing query created in last lesson assignment.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/5426831405bc9c91f4b6ced5ccd2bf27f6787809)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/1bbb2122d100d74d6b41e969de3363edbbc69cfe)
