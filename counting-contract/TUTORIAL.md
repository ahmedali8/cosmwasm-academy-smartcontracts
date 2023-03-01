# Preparing a contract to be a Dependency

Until now, we have treated our project as a standalone binary. As this is the primary way to think about the contract, sometimes we want to use it as a library crate. There are two common reasons to do so:

- Use the contract in the more complex tests to validate interactions with it
- Build the new contract based on the other one

Unfortunately, our counting contract is not ready to be used as a dependency. The main reason for that is how the entry points generation works. The `#[entry_point]` macro generates unmangled functions, being called directly by the virtual machine. The problem is that unmangled functions can cause name collisions even if they come from different crates. Therefore we want to disable entry point generation if we use the contract as a library.

Another thing missing is related to the multitest helpers. In the last lesson, we disabled its generation except for test runs. Unfortunately, the `test` predicate for conditional compilation is `true` only for the top-level tested crate and doesn't propagate to dependencies. That means that other contracts would not be able to use the utilities we deliver!

## Features

We would handle both problems using the same idea - [features!](https://doc.rust-lang.org/cargo/reference/features.html) First, up until now, our `cw-multi-test` was only a `dev-dependency`, so it wasn't built except for tests (and examples) builds.

The reason is that before, we used multitests only for tests. That changes now

- we want to compile it in also when we want to export our multitest helpers. However, we still don't want to build this dependency for normal builds - that is why we leave it to be optional:

```bash
  $ cargo add cw-multi-test --optional
```

Now we want to update `Cargo.toml` to add `features` section. It would allow us to compile some parts of our library depending on which features are chosen by user:

```toml
  [package]
  name = "counting-contract"
  version = "0.1.0"
  edition = "2021"

  [lib]
  crate-type = ["cdylib", "rlib"]

  [features]
  library = []
  tests = ["library", "cw-multi-test"]

  [dependencies]
  cosmwasm-std = "1.0.0"
  cw-multi-test = { version = "0.15.0", optional = true }
  cw-storage-plus = "0.15.0"
  schemars = "0.8.10"
  serde = { version = "1.0.144", features = ["derive"] }
  thiserror = "1.0.33"
  cosmwasm-schema = "1.1"

  [dev-dependencies]
  cw-multi-test = "0.15.0"
```

New sections define features that can be enabled in our crate. In our case, there are two of them: `library`, and `tests`. The `library` feature is meant to be enabled whenever a crate is used as a dependency. `test` feature would enable our multitest helpers and is intended to be enabled only in dev dependencies of other contracts. Additionally, you can see that the libraries have assigned some arrays to them. Those are dependencies of a particular feature. Dependency may be either an optional dependency to be enabled or another feature. Enabling a feature dependent on another feature would automatically enable it too. So you can see that the `tests` feature automatically enables the `library` feature and includes the `cw-multi-test` dependency.

Worth noting that by default, code is built with no features enabled, but this behavior can be aligned using the [default feature](https://doc.rust-lang.org/cargo/reference/features.html#the-default-feature).

## Conditional compilation

Now we want to use our features in the code. They would be predicates we would use in conditional compilation. Let's start with hiding entry points when `library` feature is enabled:

```rust
  #[cfg_attr(not(feature = "library"), entry_point)]
  pub fn instantiate(
      deps: DepsMut,
      _env: Env,
      info: MessageInfo,
      msg: InstantiateMsg,
  ) -> StdResult<Response> {
      // ...
  }

  #[cfg_attr(not(feature = "library"), entry_point)]
  pub fn execute(
      deps: DepsMut,
      env: Env,
      info: MessageInfo,
      msg: msg::ExecMsg,
  ) -> Result<Response, ContractError> {
      // ...
  }

  #[cfg_attr(not(feature = "library"), entry_point)]
  pub fn query(deps: Deps, _env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
      // ...
  }
```

We use the [#[cfg_attr(pred, attr)]](https://doc.rust-lang.org/reference/conditional-compilation.html#the-cfg_attr-attribute) attribute here. It works very similar to `#[cfg(pred)]`, but instead of enabling the following code, it inserts another attribute if the predicate is fulfilled. In our case, the predicate is `not(feature = "library")`, which reads pretty straightforward - the "library" feature is not enabled.
It is time to enable the `multitest` module if the "tests" feature is enabled. We need to improve the predicate in the `#[cfg(...)]` attribute:

```rust
  #[cfg(any(test, feature = "tests"))]
  pub mod multitest;
```

We again use the `feature = "..."` predicate, which is enabled when a feature is enabled. We pair it with an any predicate to check if `any` conditions are fulfilled (there is also a symmetrical `all` predicate).

Now there is one single step to make - let's update the `src/multitest.rs`:

```rust
  pub mod contract;
  #[cfg(test)]
  mod tests;
```

Even if one is enabling `tests` feature, there is no reason to compile it in our private test cases - we want to keep this one to build only on our test run.

When this is done, as always, it is a good point to ensure we didn't destroy any regression or contract correctness - I hope you do it after every lesson, even if I do not remind you!

## Warnings cleanup

There is one more thing we can improve on the contract. You may notice, that building it right now with `--library` feature enabled causes a warning:

````bash
  $ cargo check --features library
  # Checking counting-contract v0.1.0 (/home/hashed/confio/git/cw-academy/counting-contract) warning: unused import: `cosmwasm_std::entry_point` --> src/lib.rs:1:5 | 1 | use cosmwasm_std::entry_point; | ^^^^^^^^^^^^^^^^^^^^^^^^^ | = note: `#[warn(unused_imports)]` on by default warning: `counting-contract` (lib) generated 1 warning Finished dev [unoptimized + debuginfo] target(s) in 0.57s
The reason is that we import the `entry_point` symbol regardless of the feature state, but we never use it when `library` feature is enabled. To get rid of this warning, we can extract `entry_point` to its own `use` statement and compile it conditionally, with cfg attribute - it would make the compiler happy and less annoying!
```rust
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
````

### Code repository

[After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/3d92282218f1c9ef7aabccc6f4cc8abfafd0b4f2)
