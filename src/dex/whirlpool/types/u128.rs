// While `wasm_expose` doesn't automatically convert rust `u128` to js `bigint`, we have
// to proxy it through an opaque type that we define here. This is a workaround until
// `wasm_bindgen` supports `u128` abi conversion natively.

pub type U128 = u128;
