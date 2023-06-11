//! Re-exports of dependencies used by proc-macros
//!
//! This module is an internal implementation detail, there are absolutely no stability guarantees.
//!
//! The things here are used by the [`#[database]`](crate::database) and [`#[derive(Table)]`](liter_derive::Table) procedural macros.
//! Because the code they generate does not belong to this crate (or [`liter_derive`]), but to the crate they were invoked in, it will not have access to this [`crate`]'s (`liter`) dependencies.
//! So, whatever they use is re-exported here so that it can be accessed under the `liter` namespace.

pub use rusqlite::Result as SqlResult;
pub use const_format::{
	concatcp,
	str_repeat
};