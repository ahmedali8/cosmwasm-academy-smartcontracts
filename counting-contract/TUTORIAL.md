# The contract state

Up until now, our contract returned some fixed value while queried. It is some start, but we want smart contracts to be interactive. We must learn how to work with the smart contract state to achieve this.

Traditionally we have a helper crate to help us with that, so make use of it by adding it as a dependency:

```bash
  $ cargo add cw-storage-plus
```

[cw-storage-plus](https://docs.rs/cw-storage-plus/0.14.0/cw_storage_plus/) is a creature with specialized utilities making contract state access simple. Previously I described blockchain storage as a global key-value pair, which is true. Accessing the state directly is possible but messy, so we strongly recommend using `cw-storage-plus` in all your smart contracts.

## Defining the state

To keep things organized, let's create a separate module for the state-related things in the `lib.rs`:

```rust
  mod contract;
  pub mod msg;
  mod state;
```

Now create a new `src/state.rs` file, and create a description of our contract state in it:

```rust
  use cw_storage_plus::Item;

  pub const COUNTER: Item<u64> = Item::new("counter");
```

The contract state is defined by creating accessors to the state objects. We do not define state variables or anything like that - instead, we are creating atoms like `Item`, which would be used to access the values on the blockchain.

In particular, an [Item](https://docs.rs/cw-storage-plus/0.14.0/cw_storage_plus/struct.Item.html) is a type accessing a single object which may exist in the blockchain storage. The string passed to `Item` on instantiation is part of a key to how the data would be addressed in the blockchain. `Item` would use this value to access data, taking care of serialization and deserialization of it, so you don't need to work on raw binary data.

## Initializing the state

Now it is time to put something in the state. It would be nice if we could always assume that some counter is stored in the state. You can achieve this by storing there some default value on contract instantiation:

```rust
  use state::COUNTER;

  #[entry_point]
  pub fn instantiate(
      deps: DepsMut,
      _env: Env,
      _info: MessageInfo,
      _msg: Empty,
  ) -> StdResult<Response> {
      COUNTER.save(deps.storage, &0)?;

      Ok(Response::new())
  }
```

As you can see, storing something in the state is done by calling the [save](https://docs.rs/cw-storage-plus/0.14.0/cw_storage_plus/struct.Item.html#method.save) method on the accessor (in this case - the `Item`). The function takes two arguments.

The first one is the object implementing [Storage](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/trait.Storage.html) trait, being low-level primitive to access the underlying key-value storage. It could be used to store some data on the blockchain directly using the [set](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/trait.Storage.html#tymethod.set) function, but this is something we avoid.

The second argument is data is to be stored. The type of it corresponds to the type passed as Item generic argument.

After loading from a state, we need to do something about the potential error. The `save` function returns [StdError](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/enum.StdError.html) type, so we just need to forward it with the `?` operator.

## Loading from the state

The missing part is loading a value from the state. Hopefully, it is as simple as storing something there! Let's update the `query::value()` function:

```rust
  use cosmwasm_std::{Deps, StdResult};

  use crate::state::COUNTER;

  pub fn value(deps: Deps) -> StdResult<ValueResp> {
      let value = COUNTER.load(deps.storage)?;
      Ok(ValueResp { value })
  }
```

First, we had to make a couple of small alignments in the function signature. We need a `Deps` argument to have access to contract storage. We also need to be able to return an error in case loading from the state fails. Consequently, we also have to wrap the returned value in the `Ok(...)`.

We utilize the load function to load from the state, taking the state accessor as an argument.
Now it's time to make sure that all our tests are passing. Run `cargo test`, and then build a binary with `cargo wasm` and verify it with `cosmwasm-check` - if all are passed, your contract has an internal state!

But state on its own is not very useful if it never changes - and this is something we would take care of in the next lesson - contract executions.

## Assignment

Make it possible to start the counter with some non-zero value. To do that, you will need to create a proper instantiation message containing an initial counter value.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/30329eff1eb45920338578174cb27985ef093f47)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/722a7d88a4db41f54b5b3f37dbe3443956e84657)
