use construe::StrConstrue;

use crate::Table;
use crate::table::TableDef;

/// The set of [`Table`]s contained in a [`Database`](crate::Database)
///
/// Don't try to implement this trait manually -- use [`#[database]`](crate::database) on a tuple struct of [`Table`]s.
/// The proc macro won't just generate the [`Schema`] implementation, it will also validate it.
pub trait Schema {
	/// Represents the list of tables in the type system
	type Tables: TableList;
	/// Statically generated list of table definition structs
	const DEFINITIONS: &'static [TableDef];

	const CREATE: &'static str;
}

pub const fn define<const N: usize>(mut tables: &[&str]) -> StrConstrue<N> {
	let mut sc = StrConstrue::new();

	sc = sc.push_str("BEGIN TRANSACTION;\n");
	while let [table, rest @ ..] = tables {
		sc = sc.push_str(table);
		sc = sc.push_str("\n");
		tables = rest;
	}
	sc.push_str("END TRANSACTION;\n")
}

/// Helper trait for implementing the [`Schema`]
///
/// This trait is sealed.
/// It's implemented for nested tuples of [`Table`]s like `(TableA, (TableB, (TableC, )))` and so on.
/// This theoretically allows for [`Schema`]s with any number (> 0) of [`Table`]s, though eventually you might hit compiler limits.
///
/// Again, don't bother implementing [`Schema`] manually, use [`#[database]`](crate::database).
pub trait TableList: private::Sealed {}

impl<T: Table> TableList for (T, ) {}
impl<T: Table, L: TableList> TableList for (T, L) {}

mod private {
	use super::*;
	pub trait Sealed {}
	impl<T: Table> Sealed for (T, ) {}
	impl<T: Table, L: TableList> Sealed for (T, L) {}
}
