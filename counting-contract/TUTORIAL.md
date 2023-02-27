# Receiving funds

It is time to talk about dealing with funds in smart contracts. We will start with receiving them with the message.

## Funds and tokens

First, start with understanding what the funds and tokens are. The token is an arbitrary abstraction that an account can possess. There is some knowledge about how much of a particular token is assigned to every account. Typically it is possible to transfer tokens between accounts - the address owning them should be able to send them to someone else (however, it is not always the case).

Tokens can be implemented in a couple of ways. The most popular ones are native tokens and cw20 tokens. Today we are talking only about native tokens.

Native tokens are tokens that are managed directly by the blockchain. Native tokens can be sent to other addresses in two ways. The first one is sending a blank message to a blockchain just to transfer them. The other one is to pass the tokens as funds with a message - for example, instantiate or execute a message (as funds cannot be sent with queries).

As the whole funds sending procedure is handled by blockchain, not by the contract, there is nothing to do to accept tokens with a message. Tokens are sent to the contract right before the message is processed. If the processing succeeds, the tokens stay on the contract. The whole transaction is rolled back on execution failure, including token transfer. You can think about those tokens sent with the message as being frozen - you can assume you possess them and use them, but they would not be committed on the contract unless the whole execution transaction is processed and successful. It is essential that all processing on the CosmWasm is transactional - it makes it way easier to keep the entire blockchain state consistent.

Failing the message may seem like a way to refuse to receive funds, but it is not very reliable. Remember, there is always a way to send funds to the contract by a bank message without executing anything, and there is no way to prevent that. Therefore, designing your contract, you should never assume that you control all funds it receives.

## Reacting to funds

### Preparing the state

Maybe we can't control received funds, but we can still reasonably react to them. We will change our contract a bit, so our `Poke` message would become `Donate`, and it would expect funds to be sent with it. The contract would count all the donations with some minimum value of a particular token.

Let's start with updating the state of our contract, so it keeps the minimal donation we expect:

```rust
  use cosmwasm_std::Coin;
  use cw_storage_plus::Item;

  pub const COUNTER: Item<u64> = Item::new("counter");
  pub const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");
```

We introduced another `Item`, accessing the `Coin` value on the storage. `Coin` is a type representing a single native token amount containing a denominator (its unique identifier) and the number of tokens sent.

Now we want to update (or create an instantiation message if you don't have it yet) so we can initialize a minimal donation. We will also update the execution variant name:

```rust
  use cosmwasm_std::Coin;

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
  #[serde(rename_all = "snake_case")]
  pub struct InstantiateMsg { #[serde(default)]
      pub counter: u64,
      pub minimal_donation: Coin,
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  #[serde(rename_all = "snake_case")]
  pub enum ExecMsg {
      Donate {},
      Reset { #[serde(default)]
      counter: u64,
      },
  }
```

You need to remove the `Eq` trait from the instantiation message as it is not implemented by `Coin`. You also should update the reference to `ExecMsg::Donate`, and, ideally - the name of the `contract::exec::poke` function.
Next, update the `contract::instantiate` function:

```rust
  use crate::state::MINIMAL_DONATION;

  pub fn instantiate(deps: DepsMut, counter: u64, minimal_donation: Coin) -> StdResult<Response> {
      COUNTER.save(deps.storage, &counter)?;
      MINIMAL_DONATION.save(deps.storage, &minimal_donation)?;
      Ok(Response::new())
  }
```

And align its call in entry point:

```rust
  #[entry_point]
  pub fn instantiate(
      deps: DepsMut,
      _env: Env,
      _info: MessageInfo,
      msg: InstantiateMsg,
  ) -> StdResult<Response> {
      contract::instantiate(deps, msg.counter, msg.minimal_donation)
  }
```

### Filtering donations

Now that you have a minimal donation you want to count, time to update the `contract::exec::donate` function (previously - `poke`). What you want to do is to iterate through all the funds sent to the contract and find out if there is any which is of expected denom, and minimal amount. Hopefully, Rust standard library delivers us with the utility to do it easily:

```rust
  pub fn donate(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
      let mut counter = COUNTER.load(deps.storage)?;
      let minimal_donation = MINIMAL_DONATION.load(deps.storage)?;

      if info.funds.iter().any(|coin| {
          coin.denom == minimal_donation.denom && coin.amount >= minimal_donation.amount
      }) {
          counter += 1;
          COUNTER.save(deps.storage, &counter)?;
      }

      let resp = Response::new()
          .add_attribute("action", "donate")
          .add_attribute("sender", info.sender.as_str())
          .add_attribute("counter", counter.to_string());

      Ok(resp)
  }
```

As I mentioned in the previous lesson, funds sent with the message can be addressed using the `info` argument, its `funds` field in particular. It is a simple vector of `Coin`. To filter interesting donations, you first need to load a minimal donation from the state and then look if any of the sent funds match the predicate we discussed. Note that because I don't always want to update the counter, I delay incrementing it, making it mutable. To save gas, it would be a smart move not even to load a counter if it should not be incremented - logging it in the event is typically not a good enough reason to make a call expensive.

## Testing

As always, after adding new functionality, we want to learn how to test it. The first step is to update the test aligning it to new changes - both the instantiate and execution messages changed. I trust you can do it properly yourself, so let's just run tests after alignments. One hint I will give you is to use [cosmwasm_std::coin](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/fn.coin.html) function to create `Coin` for instantiation. On my code, one test failed:

```bash
  $ cargo test
  # Compiling counting-contract v0.1.0 (/home/hashed/confio/git/cw-academy/counting-contract) Finished test [unoptimized + debuginfo] target(s) in 1.23s Running unittests src/lib.rs (target/debug/deps/counting_contract-7de91de40c10c8b7)
  # running 3 tests test test::query_value ... ok test test::donate ... FAILED test test::reset ... ok
  # failures:
  # ---- test::donate stdout ---- thread 'test::donate' panicked at 'assertion failed: `(left == right)` left: `ValueResp { value: 0 }`, right: `ValueResp { value: 1 }`', src/lib.rs:120:9 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  # failures: test::donate
  # test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Don't worry, it is expected. Note that in the test, you didn't send any funds - but now you require some minimal donation! The entry with the particular token is expected even if you expect zero tokens. This test is handy - it tests the path of too-low donation. We would leave it here, but we will change the expected counter value to `0`:

```rust
  #[test]
  fn donate() {
      let mut app = App::default();

      let contract_id = app.store_code(counting_contract());

      let contract_addr = app
          .instantiate_contract(
              contract_id,
              Addr::unchecked("sender"),
              &InstantiateMsg {
                  counter: 0,
                  minimal_donation: coin(10, "atom"),
              },
              &[],
              "Counting contract",
              None,
          )
          .unwrap();

      app.execute_contract(
          Addr::unchecked("sender"),
          contract_addr.clone(),
          &ExecMsg::Donate {},
          &[],
      )
      .unwrap();

      let resp: ValueResp = app
          .wrap()
          .query_wasm_smart(contract_addr, &QueryMsg::Value {})
          .unwrap();

      assert_eq!(resp, ValueResp { value: 0 });
  }
```

Now let's try to add the test with a proper donation sent:

```rust
#[test]
fn donate_with_funds() {
    let mut app = App::default();

    let contract_id = app.store_code(counting_contract());

    let contract_addr = app
        .instantiate_contract(
            contract_id,
            Addr::unchecked("sender"),
            &InstantiateMsg {
                counter: 0,
                minimal_donation: coin(10, "atom"),
            },
            &[],
            "Counting contract",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked("sender"),
        contract_addr.clone(),
        &ExecMsg::Donate {},
        &coins(10, "atom"),
    )
    .unwrap();

    let resp: ValueResp = app
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Value {})
        .unwrap();

    assert_eq!(resp, ValueResp { value: 1 });
}
```

I used a `cosmwasm_std::coins` function here, which creates whole funds vector with a single token. Let's run a new test:

```bash
  $ cargo test
  # Compiling counting-contract v0.1.0 (/home/hashed/confio/git/cw-academy/counting-contract) Finished test [unoptimized + debuginfo] target(s) in 1.24s Running unittests src/lib.rs (target/debug/deps/counting_contract-7de91de40c10c8b7)
  # running 4 tests test test::donate_with_funds ... FAILED test test::query_value ... ok test test::donate ... ok test test::reset ... ok
  # failures:
  # ---- test::donate_with_funds stdout ---- thread 'test::donate_with_funds' panicked at 'called `Result::unwrap()` on an `Err` value: error executing WasmMsg: sender: sender Execute { contract_addr: "contract0", msg: Binary(7b22646f6e617465223a7b7d7d), funds: [Coin { denom: "atom", amount: Uint128(10) }] }
  # Caused by: 0: Overflow: Cannot Sub with 0 and 10 1: Cannot Sub with 0 and 10', src/lib.rs:149:10 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  # failures: test::donate_with_funds
  # test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Unfortunately - the new test is not working correctly. It fails because you try to send tokens from the "sender" to the contract, but the "sender" has no token. Multi-test is a blockchain simulator that refuses to send tokens out of nowhere. To fix this, we need to set some initial "sender" tokens balance while creating an app:

```rust
  #[test]
  fn donate_with_funds() {
      let sender = Addr::unchecked("sender");

      let mut app = App::new(|router, _api, storage| {
          router
              .bank
              .init_balance(storage, &sender, coins(10, "atom"))
              .unwrap();
      });

      let contract_id = app.store_code(counting_contract());

      let contract_addr = app
          .instantiate_contract(
              contract_id,
              Addr::unchecked("sender"),
              &InstantiateMsg {
                  counter: 0,
                  minimal_donation: coin(10, "atom"),
              },
              &[],
              "Counting contract",
              None,
          )
          .unwrap();

      app.execute_contract(
          Addr::unchecked("sender"),
          contract_addr.clone(),
          &ExecMsg::Donate {},
          &coins(10, "atom"),
      )
      .unwrap();

      let resp: ValueResp = app
          .wrap()
          .query_wasm_smart(contract_addr, &QueryMsg::Value {})
          .unwrap();

      assert_eq!(resp, ValueResp { value: 1 });
  }
```

To initialize funds, you have to change how you create an app. The [App::new](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.App.html#method.new) constructor takes an additional closure, executed right after all blockchain modules are initialized. All components are kept in the first `router` argument (a [Router](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.Router.html) type). The only one interesting for you is a `bank`([BankKeeper](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.BankKeeper.html) type) module, which allows you to set all initial balances for your account. The `api` and `storage` arguments are the same `api` and `storage` you can find in the `DepsMut` type. Now, the test should pass.

## Assignment

Fix the `donate` function, so in case of expecting `0` tokens of any denoms it works properly - counts all donations.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/bc1dd38c187c8bc6d79f7642f1e8e3317a7fd697)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/c5be8f489dc8457cc79d0e7cfcdf180f9c3daf92)
