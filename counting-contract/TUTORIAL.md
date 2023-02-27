# Sending funds

In the previous lesson, you learned how to handle funds sent with a message. Today I will tell you how to send funds owned by your contract to another address. To do so, we will prepare a new execution message - `Withdraw`, which would be intended to be called only by the contract creator. The message would send all the funds from the contract to the message sender.

## The contract creator

The first thing to do is to keep information about who created the contract. To do so, add an additional field in the state:

```rust
  use cosmwasm_std::{Addr, Coin};
  use cw_storage_plus::Item;

  pub const COUNTER: Item<u64> = Item::new("counter");
  pub const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");
  pub const OWNER: Item<Addr> = Item::new("owner");
```

and initialize it on instantiation:

```rust
  use cosmwasm_std::MessageInfo;
  use crate::state::OWNER:

  pub fn instantiate(
      deps: DepsMut,
      info: MessageInfo,
      counter: u64,
      minimal_donation: Coin,
  ) -> StdResult<Response> {
      COUNTER.save(deps.storage, &counter)?;
      MINIMAL_DONATION.save(deps.storage, &minimal_donation)?;
      OWNER.save(deps.storage, &info.sender)?;
      Ok(Response::new())
  }
```

We added the `MessageInfo` argument, so remember to update the instantiation call in the entry point. Note that I didn't add a creator into the instantiation message - we are relying on who sends the instantiation message.

## Actor model introduction

As I said in the previous lesson, one way of sending funds is to send a bank message to the blockchain. I also mentioned that all operations on CosmWasm are transactional. Let's talk about it a bit.

Communication between entities (mostly contracts) in CosmWasm is designed using an actor model. That means that contracts are performing the job end-to-end and cannot wait for other contracts or operations during operation. To communicate with the blockchain, the contract can return some messages to process in the `Response` object. All those sub-messages would be executed one by one. There is no notion of parallelism in CosmWasm. At the time, only one transaction and one message could be processed.

Now the transactional part comes into play. Message processing doesn't end with returning the `Response` object. First, all the sub-messages must be processed, which all happen in the same transaction. That means that if any of those sub-messages fail, the whole transaction is considered failing and rolled back - no state changes performed by execution or token transfers occur. This behavior can be overwritten, but this is an advanced technique we would not cover in this course.

## Sending funds

Now having a basic understanding of messages flow in CosmWasm, let's add a new execution message variant:

```rust
  #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
  #[serde(rename_all = "snake_case")]
  pub enum ExecMsg {
      Donate {},
      Reset {
          #[serde(default)]
          counter: u64,
      },
      Withdraw {},
  }
```

The new message requires the new handler in the `contract::exec` module:

```rust
  pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
      let owner = OWNER.load(deps.storage)?;
      if info.sender != owner {
          return Err(StdError::generic_err("Unauthorized"));
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

First of all, we need to check if the message sender is the one who created a contract. If it is not the case, we immediately fail execution with some generic error made from a string.

Then we need some way to figure out how much funds to send to the contract owner. We want to send all the funds, but we don't track them. Hopefully, there is a [querier](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.QuerierWrapper.html#) object on the `deps` argument, which allows us to query the blockchain for its state (even other contracts!). It may look like it breaks the described actor model, but it is not the case - query messages do not affect the blockchain in any way. Because of that, they do not obey strict transaction rules and can be called in execution. In our case, we do not query other contracts but the bank module. It would give us all the native tokens owned by the given address.

To get the contract's address, we use the [env](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Env.html) entry point argument. It contains all relevant meta information, which is not directly related to sending the message. There is current blockchain height and - what is interesting right now - the currently executed contract address.

Having funds to send, it's time to prepare the message for the blockchain. The message we are looking for is a [BankMsg](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/enum.BankMsg.html), particularly the `Send` variant. It takes a funds receiver and amount. We can add it to the `Response` using the [add_message](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Response.html#method.add_message) method.

After implementing the message, don't forget to add it to the entry point dispatching!

## Testing

Testing the new function should not be difficult - you know most of the building blocks. I recommend creating a new contract, then donating some funds, and then verifying if the funds are on the proper account - the last step would be something new:

```rust
  #[test]
  fn withdraw() {
      let owner = Addr::unchecked("owner");
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

      app.execute_contract(
          sender.clone(),
          contract_addr.clone(),
          &ExecMsg::Donate {},
          &coins(10, "atom"),
      )
      .unwrap();

      app.execute_contract(
          owner.clone(),
          contract_addr.clone(),
          &ExecMsg::Withdraw {},
          &[],
      )
      .unwrap();

      assert_eq!(
          app.wrap().query_all_balances(owner).unwrap(),
          coins(10, "atom")
      );
      assert_eq!(app.wrap().query_all_balances(sender).unwrap(), vec![]);
      assert_eq!(
          app.wrap().query_all_balances(contract_addr).unwrap(),
          vec![]
      );
  }
```

As you can see, I executed the donate operation from the address, not being an owner - it doesn't matter too much but is closer to expected on-chain usage. To verify token balances after all operations, I used the [query_all_balances](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.QuerierWrapper.html#method.query_all_balances) function - similar to the one used in the contract to get the contract balance! The `wrap` function converts an `App` to a `QuerierWrapper` and has an identical API to the `querier` object in the `deps` entry point argument.

## Assignment

Make the previously created `reset` execution to be callable only by contract owner.

Then add another operation, `WithdrawTo`, which withdraws tokens, but sends them to some address given in the message. Also add a possibility to limit how much tokens should be send this way. You should not take an address by `Addr`, it should be a `String` as sender may send any invalid addres and it should be validated first.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/21433d1efc31c1de90c511a03e9d4c8fa77c722e)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/f245b9c6729de6aa7989d2fb6eb5cdbcf4f2d5f0)
