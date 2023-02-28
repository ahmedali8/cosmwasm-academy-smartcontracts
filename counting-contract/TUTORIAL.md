# Improving multitests

The counting contract already has a bunch of tests written. The problem is that they are long (LOC-wise), and we can find many repetitions and tedious code. In this lesson, I will propose a pattern I figured out to keep unit tests more compact and very expressive in terms of the test scenarios they implement.

## The contract proxy

To simplify our tests, we would create a new type called the contract proxy. We will start creating a dedicated module for that. First, create a `src/multitest.rs` file (and add a module in `src/lib.rs`) with a single line:

```rust
  pub mod contract;
```

In the `multitest::contract` module, we would keep all the contract proxy helpers for our test. Let's start with creating a proxy type in the `src/multitest/contract.rs` file:

```rust
  use cosmwasm_std::Addr;

  pub struct CountingContract(Addr);
```

It is a typical Rust "[new type](https://doc.rust-lang.org/rust-by-example/generics/new_types.html)" pattern when we create a single tuple-struct wrapper over a type to provide an API completely different from the original one. The obvious things to add are utilities to get access to the underlying address, as we would sometimes need it to pass it around:

```rust
  impl CountingContract {
      pub fn addr(&self) -> &Addr {
          &self.0
      }
  }

  impl From<CountingContract> for Addr {
      fn from(contract: CountingContract) -> Self {
          contract.0
      }
  }
```

Note that I do not derive the `Clone` trait - it is on purpose. Semantically my contract represents a concrete contract instantiated on the blockchain, and cloning it doesn't make much sense. There may be a good idea to derive a `Debug` trait or maybe even implement [Deref<Target=Addr>](https://doc.rust-lang.org/std/ops/trait.Deref.html) trait, but I don't do it in most cases.

The next step is to provide a way to create an instance of this type. First, ask yourself a question - what is the first contract-related operation happening on the blockchain? It is loading its code! So let's add this functionality to the new contract proxy:

```rust
  use cw_multi_test::{App, ContractWrapper};
  use crate::{execute, instantiate, query};

  impl CountingContract {
      // ...

      pub fn store_code(app: &mut App) -> u64 {
          let contract = ContractWrapper::new(execute, instantiate, query);
          app.store_code(Box::new(contract))
      }
  }
```

This function is simply just a `tests::counting_contract` inlined with `App::store_code` - nothing fancy yet. One hint here - I sometimes add type being wrapper over `u64` returned by this function to signal it is a code of this very contract, but here I want to reduce boilerplate a bit. Now let's create a function to create the contract - instantiating it on an `app`:

```rust
  use cosmwasm_std::{Coin, StdResult};
  use cw_multi_test::Executor;
  use crate::InstantiateMsg;

  impl CountingContract {
      // ...

      #[track_caller]
      pub fn instantiate(
          app: &mut App,
          code_id: u64,
          sender: &Addr,
          label: &str,
          counter: impl Into<Option<u64>>,
          minimal_donation: Coin,
      ) -> StdResult<Self> {
          let counter = counter.into().unwrap_or_default();

          app.instantiate_contract(
              code_id,
              sender.clone(),
              &InstantiateMsg {
                  counter,
                  minimal_donation,
              },
              &[],
              label,
              None,
          )
          .map(CountingContract)
          .map_err(|err| err.downcast().unwrap())
      }
  }
```

Now things start to happen. To simplify calling the contract instantiation, we can eliminate any arguments we do not need for our contract. In our case - the contract does not want any funds for instantiation, so we do not need them as an argument. Also, we do not support migrations, so we set admin always to `None`. Now, look at the return type. Multitest always returns the `anyhow::Error` error, which has many benefits. Still, in our case, we know exactly an error type that can happen on instantiation - so we can immediately downcast it to make error testing more natural! Another small improvement is taking sender as a borrow instead of owned value - it seems like we lose some performance, risking unnecessary clone. Be reasonable - this extra clone on testing is completely not relevant, and how much is `&sender` easier to read than `sender.clone()` every time?

Now I want to grab your attention to the `counter` argument, which probably looks overcomplicated. First, let's consider how it is defined in the message:

```rust
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub struct InstantiateMsg {
      #[serde(default)]
      pub counter: u64,
      pub minimal_donation: Coin,
  }
```

The [#[serde(default)]](https://serde.rs/field-attrs.html#default) attribute makes this field optional in the message, which means it may be missing in the send JSON. In such a case, serde would call the [Default::default](https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default) function to get a value for the field. We want to mimic this behavior in multitest. To do so, we make our argument optional and just make it default if missing, using the [unwrap_or_default](https://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap_or_default) function.

But wait - it is not an `Option<_>`, it is some mysterious `impl Into<Option<_>>` - what is the deal here? So the [impl Trait](https://doc.rust-lang.org/rust-by-example/trait/impl_trait.html) syntax in this place means "any type which implements the given trait" - in this case, the `Into<Option<_>>` trait. We take advantage of two facts here:

- Every type implements `Into<T>` to itself - which means that `Option<T>` implements `Into<Option<T>>` (by [`From` implementation](https://doc.rust-lang.org/std/convert/trait.From.html#impl-From%3CT%3E-13))
- Every type implements `Into<Option<T>>` to itself, which means that `u64` implements `Into<Option<u64>>` (also by [`From` implementation](https://doc.rust-lang.org/std/convert/trait.From.html#impl-From%3CT%3E-13))
  As an outcome, this signature allows us to call the function passing as the `counter` either `None`, or the `u64` value - without a need to wrap it in the `Some(_)` - I like how it improves readability. The trick makes an interface a bit more complex, but if you get used to it, it makes all calls simpler.

The last thing I want you to notice is the [#[track_caller]](https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html#track_caller) attribute. It is a Rust tool to say that this is a helper function, and if there is panic in it, it should point to the place where the function is called, not where the panic occurred. So, for example - if you have a call of instantiating in the contract, and the test fails because of panic, you will not see a panic being in the `err.downcast().unwrap()` line, but instead, it would be in the line where instantiate is called in the test. I use this attribute on every test helper which contains any panicking function - it vastly improves test debugability on some strange assumption breaks.

Now let's create some execution helper:

```rust
  use crate::error::ContractError;
  use crate::msg::ExecMsg;

  impl CountingContract {
      // ...

      #[track_caller]
      pub fn donate(
          &self,
          app: &mut App,
          sender: &Addr,
          funds: &[Coin],
      ) -> Result<(), ContractError> {
          app.execute_contract(sender.clone(), self.0.clone(), &ExecMsg::Donate {}, funds)
              .map_err(|err| err.downcast().unwrap())
              .map(|_| ())
      }
  }
```

This time our helper is returning the `ContractError` on failure, as this is what our `execute` entry point returns. Also, we now want to take the funds to send them to the contract. The unclear thing here is why do I map a result, completely discarding whatever is returned by `execute_contract`? That is because I do not return anything interesting in my contract. The type returned by `execute_contract` contains all events returned by the execution and the `data` field we didn't discuss (and we want in this course - it is useful, but its usage is beyond basic). I like to return the parsed `data` field from my contract executors, as I think that testing against logs (which are execution events) is not a good practice. However, you can return an original [AppResponse](https://docs.rs/cw-multi-test/0.14.0/cw_multi_test/struct.AppResponse.html) object if you want.

Now let's implement all the missing execution helpers:

```rust
  impl CountingContract {
      // ,,,

      #[track_caller]
      pub fn reset(
          &self,
          app: &mut App,
          sender: &Addr,
          counter: impl Into<Option<u64>>,
      ) -> Result<(), ContractError> {
          let counter = counter.into().unwrap_or_default();
          app.execute_contract(
              sender.clone(),
              self.0.clone(),
              &ExecMsg::Reset { counter },
              &[],
          )
          .map_err(|err| err.downcast().unwrap())
          .map(|_| ())
      }

      #[track_caller]
      pub fn withdraw(&self, app: &mut App, sender: &Addr) -> Result<(), ContractError> {
          app.execute_contract(sender.clone(), self.0.clone(), &ExecMsg::Withdraw {}, &[])
              .map_err(|err| err.downcast().unwrap())
              .map(|_| ())
      }

      #[track_caller]
      pub fn withdraw_to(
          &self,
          app: &mut App,
          sender: &Addr,
          receiver: &Addr,
          funds: impl Into<Option<Vec<Coin>>>,
      ) -> Result<(), ContractError> {
          let funds = funds.into().unwrap_or_default();
          app.execute_contract(
              sender.clone(),
              self.0.clone(),
              &ExecMsg::WithdrawTo {
                  receiver: receiver.to_string(),
                  funds,
              },
              &[],
          )
          .map_err(|err| err.downcast().unwrap())
          .map(|_| ())
      }
  }
```

Nothing new here. The last part is to implement the query message:

```rust
  use crate::msg::{QueryMsg, ValueResp};

  impl CountingContract {
      // ...

      #[track_caller]
      pub fn query_value(&self, app: &App) -> StdResult<ValueResp> {
          app.wrap()
              .query_wasm_smart(self.0.clone(), &QueryMsg::Value {})
      }
  }
```

The query helper is simpler. We don't need a mutable `App`, as queries do not affect the blockchain state. The same is true about returned error - as nothing complex can happen in queries, we always expected `StdError` here, so there is no reason to downcast. Obviously, for queries, we need to return some reasonable data - the result of the query.

## Migrating tests

Now let's use our new utilities in tests. First, we would create a new module to keep all the tests there and have a more ordered codebase. Let's create a `src/multitest/tests.rs` file - and remember to add the module to `src/multitest.rs`. As an example, I will migrate the `donate_with_funds` test:

```rust
  use cosmwasm_std::{coin, coins, Addr};
  use cw_multi_test::App;

  use crate::msg::ValueResp;

  use super::contract::CountingContract;

  const ATOM: &str = "atom";

  #[test]
  fn donate_with_funds() {
      let sender = Addr::unchecked("sender");

      let mut app = App::new(|router, _api, storage| {
          router
              .bank
              .init_balance(storage, &sender, coins(10, ATOM))
              .unwrap();
      });

      let code_id = CountingContract::store_code(&mut app);

      let contract = CountingContract::instantiate(
          &mut app,
          code_id,
          &owner,
          "Counting contract",
          None,
          coin(10, "atom"),
      )
      .unwrap();

      contract
          .donate(&mut app, &sender, &coins(10, ATOM))
          .unwrap();

      let resp = contract.query_value(&app).unwrap();
      assert_eq!(resp, ValueResp { value: 1 });
  }
```

As you can see, at this point, performing any calls on the contracts looks like just calling functions on them. It depends on taste, but it is easier to read logic left to right with a hidden boilerplate. Our new test is reduced from 40 to 29 lines of code, so scanning it with your eyes is easier. Another improvement I made is extracting the "atom" string to its own constant - we are using it all over the place, so let the compiler help us avoid stupid typos.

The last improvement is to make the whole `multitest` module compile only when running a test - there is no point in compiling helpers or tests into the final WASM binary. Update the `src/lib.rs`:

```rust
  mod contract;
  pub mod error;
  pub mod msg;
  #[cfg(test)]
  pub mod multitest;
  mod state;
```

Now it is time to run all tests and double check the binary is a valid contract. If everything is working, remove the old `donate_with_funds` tests - we would not need it anymore.

## Assignment

Migrate all the test to use the CountingContract helper and remove them from src/lib.rs

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/09d7a4395d11364b395e1db49257d6f891aa818e)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/27d3d3fe51b8f42250f3c395df02db0669d1f007)
