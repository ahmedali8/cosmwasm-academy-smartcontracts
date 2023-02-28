# Generating Schema

Every smart contract we create has some API that the user would use to communicate with it. The API is determined by messages we create - as those are commands which can be called on the contract. However, it is not the best thing to assume that a user would be able to read the Rust code to figure out JSON messages to send. Instead, we will generate the schema of the message using a standardized JSON schema format. This way, users can use any JSON schema tool to figure out the message format.

## Preparing messages

To generate a schema, we must prepare our messages to emit metadata about them. We will use the [schemars](https://docs.rs/schemars/0.8.10/schemars/), and [cosmwasm-schema](https://docs.rs/cosmwasm-schema/1.1.2/cosmwasm_schema/) crates for that:

```bash
  $ cargo add schemars cosmwasm-schema
```

Now the first thing to do is to derive an additional JsonSchema trait on all messages, including responses:

```rust
  use cosmwasm_std::Coin;
  use schemars::JsonSchema;
  use serde::{Deserialize, Serialize};

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub struct InstantiateMsg {
      #[serde(default)]
      pub counter: u64,
      pub minimal_donation: Coin,
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub enum QueryMsg {
      Value {},
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub enum ExecMsg {
      Donate {},
      Reset {
          #[serde(default)]
          counter: u64,
      },
      Withdraw {},
      WithdrawTo {
          receiver: String,
          #[serde(default)]
          funds: Vec<Coin>,
      },
  }

  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
  #[serde(rename_all = "snake_case")]
  pub struct ValueResp {
      pub value: u64,
  }
```

I agree with you if you say those derives becoming too long and disturbing. Because of that, the [#[cw_serde]](https://docs.rs/cosmwasm-schema/1.1.2/cosmwasm_schema/attr.cw_serde.html) macro was introduced to generate all that boilerplate for you! Let's use it:

```rust
  use cosmwasm_schema::cw_serde;
  use cosmwasm_std::Coin;

  #[cw_serde]
  pub struct InstantiateMsg {
      #[serde(default)]
      pub counter: u64,
      pub minimal_donation: Coin,
  }

  #[cw_serde]
  pub enum QueryMsg {
      Value {},
  }

  #[cw_serde]
  pub enum ExecMsg {
      Donate {},
      Reset {
          #[serde(default)]
          counter: u64,
      },
      Withdraw {},
      WithdrawTo {
          receiver: String,
          #[serde(default)]
          funds: Vec<Coin>,
      },
  }

  #[cw_serde]
  pub struct ValueResp {
      pub value: u64,
  }
```

We need one final upgrade - connecting query variants with responses for particular messages. To do so, we derive the [cosmwasm_schema::QueryResponses](https://docs.rs/cosmwasm-schema/1.1.2/cosmwasm_schema/trait.QueryResponses.html) trait for the query message:

```rust
  use cosmwasm_schema::QueryResponses;

  #[cw_serde]
  #[derive(QueryResponses)]
  pub enum QueryMsg {
      #[returns(ValueResp)]
      Value {},
  }
```

The `#[returns(...)]` attribute is now required on every query variant - it describes what response type is returned for the particular query.

## Generating schema

To generate schema, we need to create a binary for that. Create a new rust file, the `src/bin/schema.rs`:

```rust
  use cosmwasm_schema::write_api;
  use counting_contract::msg::{ExecMsg, InstantiateMsg, QueryMsg};

  fn main() {
      write_api! {
          instantiate: InstantiateMsg,
          execute: ExecMsg,
          query: QueryMsg,
      }
  }
```

By putting our file in `bin` subdirectory, cargo recognizes it as an entry point for a binary. The only thing we need to do there is to execute a `write_api` macro with information about what message type is used for each entry point. Based on that, it would generate a schema file containing metainformation about the contract, JSON schemas for all messages, and the relationship between queries and their responses.

Run the binary using cargo:

```bash
  $ cargo run schema
```

You can inspect your new schema file to check what the schema files look like.

The last thing we want to add is an alias for schema generation. To make it easier to call - put it in the .`cargo/config`:

```toml
  [alias]
  wasm = "build --release --target wasm32-unknown-unknown"
  wasm-debug = "build --target wasm32-unknown-unknown"
  schema = "run schema"
```

Finally, if you are keeping your contract on a git repository, it is a good practice to add a schema directory to your `.gitignore` file, as there is no point in keeping it there - the end-user or your CI can generate schema.

## Building changes

Now let's do the final checks of the contract. We want to ensure regression is still passing:

```bash
  $ cargo test
  # Compiling counting-contract v0.1.0 (/home/hashed/confio/git/cw-academy/counting-contract) Finished test [unoptimized + debuginfo] target(s) in 1.45s Running unittests src/lib.rs (target/debug/deps/counting_contract-9c86a7074c82cada)
  # running 10 tests test test::unauthorized_reset ... ok test test::expecting_no_funds ... ok test test::donate ... ok test test::reset ... ok test test::query_value ... ok test test::unauthorized_withdraw ... ok test test::donate_with_funds ... ok test test::unauthorized_withdraw_to ... ok test test::withdraw ... ok test test::withdraw_to ... ok
  # test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

That is what we expect. Now let's build the wasm contract:

```bash
  $ cargo wasm
  # Compiling counting-contract v0.1.0 (/home/hashed/confio/git/cw-academy/counting-contract) error[E0277]: the trait bound `QueryMsg: QueryResponses` is not satisfied --> src/bin/schema.rs:5:5 | 5 | / write_api! { 6 | | instantiate: InstantiateMsg, 7 | | execute: ExecMsg, 8 | | query: QueryMsg, 9 | | } | |_____^ the trait `QueryResponses` is not implemented for `QueryMsg` | = note: this error originates in the macro `write_api` (in Nightly builds, run with -Z macro-backtrace for more info)
```

The problem is that `cargo build` we are using under the hood of `cargo wasm` does build all the targets, including binaries. Unfortunately, the `cosmwasm-schema` crate is not wasm-friendly, and the `write_api` fails to compile for this target. Hopefully - we don't need the schema binary on wasm target. We need to update our aliases again, adding a `--lib` argument for wasm targets - so only library target is built:

```toml
  [alias]
  wasm = "build --release --target wasm32-unknown-unknown --lib"
  wasm-debug = "build --target wasm32-unknown-unknown --lib"
  schema = "run schema"
```

## Code repository

[After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/f58507be4779a5d708119cdd701fa8677882b8cb)
