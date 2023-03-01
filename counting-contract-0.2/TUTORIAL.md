# Version management

We know how to migrate the contract, but we created a serious problem. Our migration works upgrading from "0.1" to "0.2", but what are we supposed to do releasing the "0.3" version of the contract with yet another set of changes? How do we know if someone is upgrading the contract version by version or jumping straight from "0.1" to "0.3"? Or maybe he migrated from "0.2" to "0.2" with no changes at all? Not to mention that maybe by mistake, the wrong contract is being migrated, and now because of admin mistake, some contract is completely unusable because it turned out from "dancing contract" to "counting contract"! Maybe it was done on purpose, but we cannot prevent nasty jokes by authorized admin - we can help him avoid mistakes.

## Storing contract version

We will store the contract version in the contract state to solve our problem. Then, we would be able to get it back on the contract migration to determine how to migrate the contract. However, we won't do it directly. We will use the standardized cw2 tool for that. Let's add it to the dependencies:

```bash
  $ cargo add cw2
  $ cargo add cw2
```

We usually do not change the code of the previous contract version. This is why it is important always to add versioning in the earliest version of the contract, so when you need it in the future, you will have it there. In the case you forgot to do that, there is a way around it - you can assume that if there is no version stored, it is the first released version. It is, however, not the best idea, as if you would forget it again to add it on the first migration, you will be in a very bad spot not being able to distinguish those two versions.

Now let's update our instantiate to store the contract version in its state when the contract is created (do it in both contract versions):

```rust
  use cw2::set_contract_version;

  const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
  const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

  pub fn instantiate(
      deps: DepsMut,
      info: MessageInfo,
      counter: u64,
      minimal_donation: Coin,
  ) -> StdResult<Response> {
      set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

      STATE.save(
          deps.storage,
          &State {
              counter,
              minimal_donation,
              owner: info.sender,
          },
      )?;
      Ok(Response::new())
  }
```

Notice the two constants I created for this. I am using [env!](https://doc.rust-lang.org/std/macro.env.html) macro to reach to environment variables [created by cargo](https://doc.rust-lang.org/cargo/reference/environment-variables.html). They are set to the contract name and version defined in `Cargo.toml`. This way, we don't need to track the version in multiple places.

You may argue that storing contract names is unnecessary, but it has its uses. It prevents the admin from updating the other contract he intended to - we can detect contract name collision and refuse to migrate different contracts.

## Dispatching migration

We finished the generic changes. Now everything should be done only in `0.2` version of the contract. First, we need to add two more error variants to our `ContractError` - we want to report two failing situations on contract migration:

```rust
  #[derive(Error, Debug, PartialEq)]
  pub enum ContractError {
      #[error("{0}")]
      Std(#[from] StdError),

      #[error("Unauthorized - only {owner} can call it")]
      Unauthorized { owner: String },

      #[error("Invalid contract to migrate from: {contract}")]
      InvalidContract { contract: String },

      #[error("Unsupported contract version for migration: {version}")]
      InvalidContractVersion { version: String },
  }
```

Now Let's rename the `contract::migrate` function to `contract::migrate_0_1_0`. This migration is valid for migration from a particular contract version, and in the future, you can have a function to migrate from every version separately. It would also be a great idea to keep all of this in its own `migration` module. Then create another `migrate` function, performing the version dispatch:

```rust
  pub fn migrate(mut deps: DepsMut) -> Result<Response, ContractError> {
      let contract_version = get_contract_version(deps.storage)?;

      if contract_version.contract != CONTRACT_NAME {
          return Err(ContractError::InvalidContract {
              contract: contract_version.contract,
          });
      }

      let resp = match contract_version.version.as_str() {
          "0.1.0" => migrate_0_1_0(deps.branch()).map_err(ContractError::from)?,
          CONTRACT_VERSION => return Ok(Response::default()),
          version => {
              return Err(ContractError::InvalidContractVersion {
                  version: version.into(),
              })
          }
      };

      set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

      Ok(resp)
  }
```

First, we have to load the version of the contract on-chain. Then we validate if the contract name didn't change to prevent an admin mistake. If the contract to update is proper, we dispatch to a migrate functions. Note that I am explicitly checking for the current contract version, in which case, I don't want to do anything - we return immediately. It is worth noting it works only if the `CONTRACT_VERSION` is a constant - if it is a variable, it would be treated as a generic match, and the last branch would be unreachable. If the contract version is not matched on the final branch, we refuse to migrate. It might happen when the contract is downgraded - but we don't know how to downgrade it, as we cannot predict any state changes! If you want to allow this, then you can just do nothing in such a case, but it may lead to strange behavior, so make sure you properly design for such a case.

Also, note the [branch()](https://doc.rust-lang.org/cargo/reference/environment-variables.html) function we call on `deps` - it is a useful utility which allows having another copy of a mutable state in a single contract - like a clone().

Finally, we updated the contract version to the new value so the contract version would be valid on future migration. Last little thing - ensure you updated your entry point signature to return the proper error type. Now run your whole regression and contract check - it should still pass.

## Assignment

Test that migrating the contract to the same version works.

### Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/56e0a3f1012dcc338973251c024ee30ddafbe370)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/53d1ba614cd8923d08115ffee0d7f3c0f571f154)
