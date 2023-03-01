# Migrations

We have a contract that we can consider ready to deploy. Now we want to learn how to make it possible to update a contract on-chain when we would do some updates.

To do so, we need to implement the migration entry point for the contract. Migration is a specialized entry point, which is called when the contract is requested to update to another code version. The migration purpose is to align the state to the new schema.

To understand it better, let's think of the example. Imagine that, for some reason, you decided to keep the counter and minimal donation in a single `Item`, coupling them together in the struct. When you just upgrade the contract, you will have a problem. In the underlying kv storage, there would be some `counter` data with serialized `u64`, and the `minimal_donation` with JSON representing `Coin`, but there would be no entry for our new structure. Every time you try to reach for that, there is no data to read. And this is where migrations come into play - we want to call a one-time function that reads the old data structures which are no longer used and stores them in the new format. Also, migration can receive additional data in the message, so it can, for instance, have info about the default value for the newly added field.

There is another usage of migrations I met - the contract reconfiguration. As it is guaranteed by blockchain that only a particular address (admin) can call the migration, he can perform the migration to the same code that the contract used before. This call could take a new configuration (like duplication of instantiation message) and apply it to the contract. I've seen this once, but I do not recommend this - you can quickly implement it with execution, restricting the call for a dedicated address.

## Preparing the contract

Before creating the updated contract version, we must perform some alignments in our test helpers. Until now, we always set the `admin` argument of `insantiate_contract` to `None`. The reason was that we didn't support migrations, so we didn't bother about admin. Today it changes - we will need to test if our migration entry point works. Therefore we need to be able to set an admin on the old contract version when we instantiate it. Let's update the `CountingContract::instantiate` function:

```rust
  impl CountingContract {
      #[track_caller]
      pub fn instantiate<'a>(
          app: &mut App,
          code_id: u64,
          sender: &Addr,
          label: &str,
          admin: impl Into<Option<&'a Addr>>,
          counter: impl Into<Option<u64>>,
          minimal_donation: Coin,
      ) -> StdResult<Self> {
          let admin = admin.into();
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
              admin.map(Addr::to_string),
          )
          .map(CountingContract)
          .map_err(|err| err.downcast().unwrap())
      }
  }
```

As you notice, I used my `impl Into<Option<_>>` trick here. I had to pass an additional lifetime - it is a Rust thing that in this trait implementing context, it is not the best with lifetime elision, and you have to pass it some concrete lifetime, but it can be just a generic one, and it should work.

Now you need to align all our tests - all of them are calling instantiate, so an additional argument has to be passed.

## Updating the contract

Now we want to create a new version of our contract. However, we still need access to the previous one, so we would be able to use it in the migration test. To achieve that now, let's copy a whole contract to the new project:

```bash
  $ cp -R ./counting-contract/ ./counting-contract-0.2
```

Let's clarify - it is not a proper way to create a new contract version. Typically we keep our contracts on some git repository and tag every contract release. Also, the contract may be uploaded on crates.io or another repository, so we have a simple way to relate it to an older version. The only reason I suggest copying the entire contract is to make it slightly simpler, not going into how you keep your contract on your machine. Still, I assume you use some proper version control in a real-life environment.

Now, let's update the `Cargo.toml` of our new contract version:

```toml
  [package]
  name = "counting-contract"
  version = "0.2.0"
  edition = "2021"
```

We need to update the contract version, as we will use both versions in a single test, and cargo would refuse to have two different copies of the same version of the crate. Also - it is good to track the versioning of your software properly.

Now let's make some breaking contract change. The example I mentioned before, was a nice one so let's update the `src/state.rs`:

```rust
  use cosmwasm_std::{Addr, Coin};
  use cw_storage_plus::Item;
  use serde::{Deserialize, Serialize};

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
  pub struct State {
      pub counter: u64,
      pub minimal_donation: Coin,
  }

  pub const STATE: Item<State> = Item::new("state");
  pub const OWNER: Item<Addr> = Item::new("owner");
```

Now we need to update our handlers, starting with instantiate:

```rust
  pub fn instantiate(
      deps: DepsMut,
      info: MessageInfo,
      counter: u64,
      minimal_donation: Coin,
  ) -> StdResult<Response> {
      STATE.save(
          deps.storage,
          &State {
              counter,
              minimal_donation,
          },
      )?;
      OWNER.save(deps.storage, &info.sender)?;
      Ok(Response::new())
  }
```

Then query:

```rust
  pub mod query {
      use cosmwasm_std::{Deps, StdResult};

      use crate::msg::ValueResp;
      use crate::state::STATE;

      pub fn value(deps: Deps) -> StdResult<ValueResp> {
          let value = STATE.load(deps.storage)?.counter;
          Ok(ValueResp { value })
      }
  }
```

And finally execute:

```rust
  pub mod exec {
      use cosmwasm_std::{BankMsg, Coin, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};

      use crate::error::ContractError;
      use crate::state::{OWNER, STATE};

      pub fn donate(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
          let mut state = STATE.load(deps.storage)?;

          if state.minimal_donation.amount.is_zero()
              || info.funds.iter().any(|coin| {
                  coin.denom == state.minimal_donation.denom
                      && coin.amount >= state.minimal_donation.amount
              })
          {
              state.counter += 1;
              STATE.save(deps.storage, &state)?;
          }

          let resp = Response::new()
              .add_attribute("action", "poke")
              .add_attribute("sender", info.sender.as_str())
              .add_attribute("counter", state.counter.to_string());

          Ok(resp)
      }

      pub fn reset(
          deps: DepsMut,
          info: MessageInfo,
          counter: u64,
      ) -> Result<Response, ContractError> {
          let owner = OWNER.load(deps.storage)?;
          if info.sender != owner {
              return Err(ContractError::Unauthorized {
                  owner: owner.to_string(),
              });
          }

          STATE.update(deps.storage, |mut state| -> StdResult<_> {
              state.counter = counter;
              Ok(state)
          })?;

          let resp = Response::new()
              .add_attribute("action", "reset")
              .add_attribute("sender", info.sender.as_str())
              .add_attribute("counter", counter.to_string());

          Ok(resp)
      }

      // Withdraws unthouched
  }
```

## Testing

We usually talk about testing new things after the main feature, but this time let's spice things a bit and follow the TDD way of work. Before implementing the migration, we will write the test for it to see the problem occurring when migration is missing.

We would need to use the old contract with the enabled tests feature for our tests, so let's start with that. We can use cargo add for this:

```bash
  $ cargo add counting-contract \ --rename counting-contract-0_1 --path ../counting-contract --features tests --dev
```

We would need to use the old contract with the enabled `tests` feature for our tests, so let's start with that. We can use `cargo add` for this:

Complicated command. First of all, we need to rename our dependency. Cargo will not allow having two dependencies of the same name. We also need to tell where to use the dependency from - here, I passed a `--path` to take dependency from the filesystem, but typically I will use a `--git` flag to point to some git tag (or I will use `counting-contract@0.1` as name, to take the older version from `crates.io`). We also added the required feature (the "library" feature will be enabled automatically), and specify a dependency to be only used for development (tests and examples).

Now we need to add the helper for the migration in our `CountingContract` utility:

```rust
  impl CountingContract {
      // ...
      #[track_caller]
      pub fn migrate(app: &mut App, contract: Addr, code_id: u64, sender: &Addr) -> StdResult<Self> {
          app.migrate_contract(sender.clone(), contract.clone(), &Empty {}, code_id)
              .map_err(|err| err.downcast().unwrap())
              .map(|_| Self(contract))
      }
  }
```

I decided to implement it as a static function, as we don't want to modify the previous contract helper. I could do that using the extension trait, but it would be more work than the benefit it gives, so let's keep it this way. After all, we return the contract address wrapped in the new helper. Finally, let's create a test:

```rust
  use crate::state::{State, STATE};
  use counting_contract_0_1::multitest::contract::CountingContract as CountingContract_0_1;

  #[test]
  fn migration() {
      let admin = Addr::unchecked("admin");
      let owner = Addr::unchecked("owner");
      let sender = Addr::unchecked("sender");

      let mut app = App::new(|router, _api, storage| {
          router
              .bank
              .init_balance(storage, &sender, coins(10, "atom"))
              .unwrap();
      });

      let old_code_id = CountingContract_0_1::store_code(&mut app);
      let new_code_id = CountingContract::store_code(&mut app);

      let contract = CountingContract_0_1::instantiate(
          &mut app,
          old_code_id,
          &owner,
          "Counting contract",
          &admin,
          None,
          coin(10, ATOM),
      )
      .unwrap();

      contract
          .donate(&mut app, &sender, &coins(10, ATOM))
          .unwrap();

      let contract =
          CountingContract::migrate(&mut app, contract.into(), new_code_id, &admin).unwrap();

      let resp = contract.query_value(&app).unwrap();
      assert_eq!(resp, ValueResp { value: 1 });

      let state = STATE.query(&app.wrap(), contract.addr().clone()).unwrap();
      assert_eq!(
          state,
          State {
              counter: 1,
              minimal_donation: coin(10, ATOM)
          }
      );
  }
```

I think the test is straightforward to read, but I want to bring your attention to two details. First, look at how did I import the `CountingContract` from the old crate version, renaming it. It is way more convenient than using the whole path whenever you need it. And obviously, I cannot just import both `CountingContract`s as it would cause a name collision.

The second thing is how I reach the contract state, querying it using the [Item::query](https://docs.rs/cw-storage-plus/0.14.0/cw_storage_plus/struct.Item.html#method.query) function. This technique is called "raw queries", and it allows to reach the contract state, which is public by design. There is no hermetization in blockchain storage by its design, and sophisticated techniques based on [Zero Knowledge Proofs](https://en.wikipedia.org/wiki/Zero-knowledge_proof) are needed to hide any information. That is, however, far beyond this course scope.

You can also use raw queries in your smart contract code - to reach the state of other contracts! It is sometimes useful, as it is typically cheaper than normal ("smart") queries, but it has problems - the internal contract state may change more often than its message API. Anyway - designing the multi-contract system is something I mentioned in the last lesson, but we will not go deeply into that.

Now let's run the new test:

```bash
  counting-contract-0.2 $ cargo test migration
  # warning: unused import: `entry_point` --> /home/hashed/confio/git/cw-academy/counting-contract/src/lib.rs:2:5 | 2 | entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, | ^^^^^^^^^^^ | = note: `#[warn(unused_imports)]` on by default warning: `counting-contract` (lib) generated 1 warning Compiling counting-contract v0.2.0 (/home/hashed/confio/git/cw-academy/counting-contract-0.2) Finished test [unoptimized + debuginfo] target(s) in 1.23s Running unittests src/lib.rs (target/debug/deps/counting_contract-e4d3800e7a53c37f) running 1 test test multitest::tests::migration ... FAILED failures: ---- multitest::tests::migration stdout ---- thread 'multitest::tests::migration' panicked at 'called `Result::unwrap()` on an `Err` value: error executing WasmMsg: sender: admin Migrate { contract_addr: "contract0", new_code_id: 2, msg: Binary(7b7d) } Caused by: migrate not implemented for contract', src/multitest/contract.rs:51:43 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace failures: multitest::tests::migration
```

We have the failure caused by a lack of migration implementation. Let's add the new migration entry point in `src/lib.rs`:

```rust
  #[cfg_attr(not(feature = "library"), entry_point)]
  pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
      Ok(Response::new())
  }
```

Taking it step by step, we start with `migrate` doing nothing, just to have it implemented. Let's rerun the test:

```bash
  $ cargo test migration
  #warning: unused import: `entry_point` --> /home/hashed/confio/git/cw-academy/counting-contract/src/lib.rs:2:5 | 2 | entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, | ^^^^^^^^^^^ | = note: `#[warn(unused_imports)]` on by default warning: `counting-contract` (lib) generated 1 warning Compiling counting-contract v0.2.0 (/home/hashed/confio/git/cw-academy/counting-contract-0.2) Finished test [unoptimized + debuginfo] target(s) in 1.23s Running unittests src/lib.rs (target/debug/deps/counting_contract-e4d3800e7a53c37f) running 1 test test multitest::tests::migration ... FAILED failures: ---- multitest::tests::migration stdout ---- thread 'multitest::tests::migration' panicked at 'called `Result::unwrap()` on an `Err` value: error executing WasmMsg: sender: admin Migrate { contract_addr: "contract0", new_code_id: 2, msg: Binary(7b7d) } Caused by: migrate not implemented for contract', src/multitest/contract.rs:51:43 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace failures: multitest::tests::migration
```

And still unimplemented migration?! But believe me, it's all OK - we never told multitest where to find our migration! Let's update the `CountingContract::store_code`, to solve this:

```rust
  pub fn store_code(app: &mut App) -> u64 {
      let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
      app.store_code(Box::new(contract))
  }
```

And rerun the test once more:

```bash
  $ cargo test migration
  # warning: unused import: `entry_point` --> /home/hashed/confio/git/cw-academy/counting-contract/src/lib.rs:2:5 | 2 | entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, | ^^^^^^^^^^^ | = note: `#[warn(unused_imports)]` on by default warning: `counting-contract` (lib) generated 1 warning Compiling counting-contract v0.2.0 (/home/hashed/confio/git/cw-academy/counting-contract-0.2) Finished test [unoptimized + debuginfo] target(s) in 1.28s Running unittests src/lib.rs (target/debug/deps/counting_contract-e4d3800e7a53c37f) running 1 test test multitest::tests::migration ... FAILED failures: ---- multitest::tests::migration stdout ---- thread 'multitest::tests::migration' panicked at 'called `Result::unwrap()` on an `Err` value: GenericErr { msg: "Querier contract error: counting_contract::state::State not found" }', src/multitest/tests.rs:347:43 note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace failures: multitest::tests::migration
```

We still have a failure, but this time it is different. When calling the query, the contract tries to reach the state variable - but it was never stored. We need to fix it. Migration is the perfect place to do so.

## Implementing migration

Implementing migration would be very similar to implementing the instantiation, with the difference that most of the data would be loaded from the old state instead of incoming messages. Here is our handler from `src/contract.rs`:

```rust
  pub fn migrate(deps: DepsMut) -> StdResult<Response> {
      const COUNTER: Item<u64> = Item::new("counter");
      const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");

      let counter = COUNTER.load(deps.storage)?;
      let minimal_donation = MINIMAL_DONATION.load(deps.storage)?;

      STATE.save(
          deps.storage,
          &State {
              counter,
              minimal_donation,
          },
      )?;

      Ok(Response::new())
  }
```

You should first notice that I have to recreate my old contract state to reach that. The other way to attempt that is to use an old contract version as a dependency, but I don't like it, as it causes loads of old contract versions as dependencies at some point, and all of that is just for migration called, probably once per version. However, if you prefer this approach, it won't be wrong - but in such case, I would add a dedicated feature flag, which would export only the state and nothing else (and it would probably be a dependency for a regular "library" flag).

Also worth noting is that the `migrate` entry point doesn't take funds or message info. However, it still is a part of an actor model flow - returning `Response` it can send messages to the blockchain and works in a transactional way.

Now it is time for a final test rerun and contract check. This time everything should pass.

## Assignment

Move the "owner" to be kept in the same structure as "counter" and "minimal donation" in the contract.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/5315bcad1bb996f131d4b9992c50b48ed7ec347b)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/49e9aa1564cc6e602880bef1530313432d1fd53f)
