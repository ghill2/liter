//! Various meta-programming traits for [`Schema`](crate::Schema) validation
//!
//! None of the traits in this module should be implemented manually, and many are sealed.
//! They exist to validate [`Schema`](crate::Schema)s at compile-time: making sure that types can only hold (foreign-key) [`Ref`](crate::Ref)erences to types that are in the same [`Schema`](crate::Schema).
//!
//! Here's an example of the type of this type of error being spotted.
//!```compile_fail
//! use liter::{database, Ref, Table};
//!
//! /// This struct is not in `ExampleDB`
//! #[derive(Table)]
//! struct A {
//!     #[key]
//!     id: i64,
//!     version: u8,
//!     data: String
//! }
//!
//! /// This one is, but references the struct that isn't
//! #[derive(Table)]
//! struct B {
//!     #[key]
//!     id: i64,
//!     reference_to_a: Ref<A>
//! }
//!
//! #[database]    // ← This fails because `B` requires that `A` is in the DB
//! struct ExampleDB (
//!     B
//! );
//! ```
//!
//! Note that it's not an error to define the structs `A` and `B` by themselves, since they don't know what [`Schema`](crate::Schema)s they will be a part of, `#[derive(Table)]` on `A` and `B` compiles just fine.
//! Only when we try to define an invalid database does the issue arise.

pub mod filter;
pub use filter::Filtered;
pub mod tuple;
pub mod validate;
pub use validate::{
	IsValidFor,
	PartOf
};
