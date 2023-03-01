# Counting contract (cosmwasm)

```bash
  # Pre-requisites:
  $ cargo install cosmwasm-check

  # create new cargo library Or you can clone this repo
  $ cargo new --lib counting_contract

  # build
  $ cargo build --release --target wasm32-unknown-unknown

  ## Alternatively we can create config in "./.cargo/config":
  # [alias]
  # wasm = "build --release --target wasm32-unknown-unknown"
  # wasm-debug = "build --target wasm32-unknown-unknown"

  # and now we can use cargo wasm to build
  $ cargo wasm

  # validate if a Wasm binary is a valid CosmWasm smart contract
  $ cosmwasm-check ./target/wasm32-unknown-unknown/release/counting_contract.wasm
```
