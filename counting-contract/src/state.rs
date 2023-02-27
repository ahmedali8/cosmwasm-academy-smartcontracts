use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

// Create a constant COUNTER of type Item<u64> and initialize it with a new Item instance
// The new() method takes a string argument which is used as the storage key for this item
pub const COUNTER: Item<u64> = Item::new("counter"); // here "counter" is storage_key

// Create a constant MINIMAL_DONATION of type Item<Coin> and initialize it with a new Item instance
// The new() method takes a string argument which is used as the storage key for this item
pub const MINIMAL_DONATION: Item<Coin> = Item::new("minimal_donation");

// Create a constant OWNER of type Item<Addr> and initialize it with a new Item instance
// The new() method takes a string argument which is used as the storage key for this item
pub const OWNER: Item<Addr> = Item::new("owner");
