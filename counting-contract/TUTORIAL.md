# Installing the toolset

Before we start building our CosmWasm smart contract, we need to set up an environment. The first thing you need is cargo, the Rust stack. You can find its installation instructions [here](https://www.rust-lang.org/tools/install).

Additionally, you need a wasm target for cargo, which you can install with `rustup target add wasm32-unknown-unknown`. It would allow you to cross-compile Rust code to wasm bytecode instead of native, which we need for our smart contracts.

When your Rust is set up, you should install the [cosmwasm-check](https://crates.io/crates/cosmwasm-check) utility. It is a tool verifying if the wasm binary is a valid CosmWasm smart contract ready to upload to the chain. We want all contracts we build to be valid, so `cosmwasm-check` would be very useful. You can install it using cargo, executing `cargo install cosmwasm-check` in your terminal.

# Prepare a project

Now, having the environment ready, we can create a new smart contract project using cargo. For convenience, create a new directory for this course - I will call it cw-course in examples:

```bash
  $ mkdir cw-course
  $ cd cw-course
```

Now create a new cargo library in this folder:

```bash
  $ cargo new --lib ./counting*contract
```

It is best to create smart contract projects as libraries, as in the end, the output we care about is a dynamic wasm library. Now you can go to your project directory and try to build it:

```bash
  $ cd ./counting_contract counting_contract
  $ cargo build
```

Assuming you set up your environment correctly, the build should pass. All your build artifacts are located in `target` directory. You may want to look for some `*.wasm` files in there - let's check if anything is generated:

```bash
  $ find ./target -name "*.wasm"
```

No `*.wasm` files are built at this point, and there are two reasons for that. First, as I said previously - we are interested in dynamic library output, while by default, Rust is building statically linked libraries. Second - as we didn't tell cargo to build a Wasm output, it, by default, created a native binary for your machine. Let's handle those problems one by one.

## Building a Wasm artifact

To build dynamic libraries in cargo, you must modify your `Cargo.toml` file. Add a `lib` section setting a `crate-type` to `cdylib` which is proper for dynamic libraries. Your `Cargo.toml` should look like this:

```toml
  [package]
  name = "counting-contract"
  version = "0.1.0"
  edition = "2021"

  [lib]
  crate-type = ["cdylib", "rlib"]
```

Note that in addition to `cdylib` I added a `rlib` crate type. It instructs cargo to build two kinds of outputs, and the `rlib` is a standard static rust library. We do not need it right now, but it is helpful to have it, so we want to leave it here for now.

Now you are ready to build your wasm output by calling a slightly modified build command:

```bash
  $ cargo build --target wasm32-unknown-unknown
  $ find ./target -name "*.wasm"
```

You have your wasm binary ready at this point, but I will agree if you tell me that the building command is a bit tedious. Hopefully, there is a simple way to make it more compact. We often do that by creating a `.cargo/config` file in a smart contract project, looking like this:

```toml
  [alias]
  wasm = "build --release --target wasm32-unknown-unknown"
  wasm-debug = "build --target wasm32-unknown-unknown"
```

This file creates two aliases for `cargo` utility - `wasm` for building a release wasm binary, and `wasm-debug` for a debug `wasm` output. I use a release output by default which may be surprising as it doesn't match typical cargo behaviour, but I have a reason. I will mostly use my wasm binaries as final artifacts to upload to the blockchain, but I will not debug them. If I had to debug a smart contract, I would create a Rust test for it and debug it in the native target with `lldb` instead of struggling with debugging the Wasm binary. That being said, I don't think I ever faced a situation where I needed to debug symbols in Wasm output, but I always want my Wasm output to be as small as possible. Having all of those in mind - I build release wasm output by default, and I leave a `wasm-debug` alias just in case I need to debug Wasm build. Also, as the release optimized build is a bit slower than debug one, for regular checks if contract builds, I would use `cargo build` (or even `cargo check`) command. I will use `cargo wasm` only when I want to ensure that my contract is ready to be uploaded to the blockchain. When I want a proper final binary, I would use yet another tool - [Rust optimizer](https://github.com/CosmWasm/rust-optimizer), but I will not do this in this course, so it is out of scope. Now, let's check if our alias works:

```bash
  $ cargo wasm
  $ find ./target -name "*wasm"
```

We have new artifacts - our release Wasm build. Now its time for a final check -let's validate if a Wasm binary is a valid CosmWasm smart contract:

```bash
  $ cosmwasm-check ./target/wasm32-unknown-unknown/release/counting_contract.wasm
```

## Creating entry points

It seems like something is wrong; unfortunately, an error message is not very helpful here. An actual reason for the failure is that the contract has no entry points, and the `instantiate` entry point is required for working. The reason why `cosmwasm-check` is complaining about some version markers is that the very same macro generates the marker as the entry point, so adding one should solve this problem.

First, we need to add a [cosmwasm-std](https://crates.io/crates/cosmwasm-std) dependency to the project. Assuming you have at least 1.62 rust version (or you manually installed cargo-edit utility), you can use cargo add:

```bash
  $ cargo add cosmwasm-std
```

If you are using older Rust version, you have to update your `Cargo.toml` manually:

```toml
[package]
name = "counting-contract"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cosmwasm-std = "1.0.0"
```

The next step is to create an entry point in `src/lib.rs`:

```rust
  use cosmwasm_std::{
      DepsMut, Env, MessageInfo, Empty, StdResult, Response, entry_point
  };

  #[entry_point]
  pub fn instantiate(
      _deps: DepsMut,
      _env: Env,
      _info: MessageInfo,
      _msg: Empty,
  ) -> StdResult<Response> {
      Ok(Response::new())
  }
```

Let's talk a bit about this. The entry point is the first function called by CosmWasm virtual machine when action is performed on a smart contract. It is like a `main` function in a regular Rust application. The important difference is that, unlike native binaries, smart contracts have multiple entry points. The `instantiate` one is called when the smart contract is created for the first time - you can think about it as it is a constructor for a contract. Also, the signature of CosmWasm entry points differs from `main` function. Let's start with explaining `instantiate` arguments:

- [deps: DepsMut](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.DepsMut.html) is a utility type for communicating with the outer world - it allows querying and updating the contract state, querying another contract state, and gives access to an Api object with a couple of helpers functions for dealing with CW addresses.

- [env: Env](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Env.html) is an object representing the blockchains state when executing the message - the chain height and id, current timestamp, and the called contract address.

- [info: MessageInfo](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.MessageInfo.html) contains metainformation about the message which triggered an execution - an address that sends the message and chain native tokens sent with the message.

- [msg: Empty](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Empty.html) is the message triggering execution itself - for now, it is the Empty type that represents `{}` JSON, but the type of this argument can be anything that is deserializable, and we will pass more complex types here in the future.

Notice an essential attribute decorating our entry point [#[entry_point]](https://docs.rs/cosmwasm-std/1.1.2/cosmwasm_std/attr.entry_point.html). Its purpose is to wrap the whole entry point to the form Wasm runtime understands. The proper Wasm entry points can use only basic types supported natively by Wasm specification, and Rust structures and enums are not in this set. Working with such entry points would be rather overcomplicated, so CosmWasm creators delivered the entry_point macro. It creates the raw Wasm entry point, calling the decorated function internally and doing all the magic required to build our high-level Rust arguments from arguments passed by Wasm runtime.

The next thing to look at is the return type. I used [StdResult<Response>](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/type.StdResult.html) for this simple example, which is an alias for `Result<Response, StdError>`. The return entry point type would always be a [Result](https://doc.rust-lang.org/std/result/enum.Result.html) type, with some error type implementing [ToString](https://doc.rust-lang.org/std/string/trait.ToString.html) trait and a well-defined type for success case. For most entry points, an `Ok` case would be the [Response](https://docs.rs/cosmwasm-std/1.0.0/cosmwasm_std/struct.Response.html) type.

Having that, I need a body for the function. As I want it to do nothing, I just return a default created `Response` object.

Now let's compile the project and make sure it passes the `cosmwasm-check`:

```bash
  $ cargo wasm
  $ cosmwasm-check ./target/wasm32-unknown-unknown/release/counting_contract.wasm
```

As everything passes, you have your first proper CosmWasm smart contract ready. It is not very useful yet - it can only be created, but you will improve it during the following lessons.

## Assignment

As a practice after this lesson try to add two more entry points - `query` and `execute` using [#[entry_point]](https://docs.rs/cosmwasm-std/1.1.2/cosmwasm_std/attr.entry_point.html) documentation. For now entry points should do nothing for now.
Code repository

- [After the lesson](https://github.com/CosmWasm/cw-academy-course/commit/7d007d4833530c3f7464f1e304749715e5c4d2f3)
- [After the assignment](https://github.com/CosmWasm/cw-academy-course/commit/fd2ecba327cb3a2590de122ba2cd8f86c731d7c2)
