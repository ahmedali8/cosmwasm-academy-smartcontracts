// Define a new module called `query`
pub mod query {
    // Import the `ValueResp` struct from the `msg` module
    use crate::msg::ValueResp;

    // Define a public function called `value` that takes no arguments and returns a `ValueResp` struct
    pub fn value() -> ValueResp {
        // Create a new `ValueResp` struct with a `value` field set to 0 and return it
        ValueResp { value: 0 }
    }

    // Define a public function called `incremented` that takes a single `u64` argument and returns a `ValueResp` struct
    pub fn incremented(value: u64) -> ValueResp {
        // Create a new `ValueResp` struct with a `value` field set to the input `value` incremented by 1, and return it
        ValueResp { value: value + 1 }
    }
}
