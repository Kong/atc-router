#![deny(warnings, missing_debug_implementations)]
// Transitive dev-dependencies (e.g. criterion vs. pest_derive) can pull in
// two major versions of a proc-macro crate like `syn` independently of any
// change here; this isn't something a single PR controls, so don't let it
// fail the build.
#![allow(clippy::multiple_crate_versions)]
/*!
This crate provides a powerful rule based matching engine that can match a set of routes
against dynamic input value efficiently.
It is mainly used inside the [Kong Gateway](https://github.com/Kong/kong)
for performing route matching against incoming requests and is used as a FFI binding
for LuaJIT.

Please see the [repository README.md](https://github.com/Kong/atc-router/blob/main/README.md)
for more detailed explainations of the concepts and APIs.

# Crate features

* **ffi** -
  Builds the FFI based interface which is suitable for use by a foreign language such as
  C or LuaJIT. This feature is on by default.
* **serde** -
  Enable serde integration which allows data structures to be serializable/deserializable.
*/

pub mod ast;
pub mod context;
pub mod interpreter;
pub mod parser;
pub mod router;
pub mod schema;
pub mod semantics;

#[cfg(feature = "ffi")]
pub mod ffi;

#[macro_use]
extern crate pest_derive;
