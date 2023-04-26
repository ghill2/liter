use crate::Table;

/// The set of [`Table`]s contained in a [`Database`](crate::Database)
///
/// Don't try to implement this trait manually -- use [`#[database]`](crate::database) on a tuple struct of [`Table`]s.
/// The proc macro won't just generate the [`Schema`] implementation, it will also validate it.
pub trait Schema {
	type Tables: TableList;
	type AllValues;

	fn define() -> String {
		let mut table_defs = Vec::new();
		Self::Tables::assemble_into(&mut table_defs);
		let tables = table_defs.join("\n");
		format!("BEGIN TRANSACTION;\n{tables}\nEND TRANSACTION;\n")
	}
}

/// Helper trait for implementing the [`Schema`]
///
/// This trait is sealed.
/// It's implemented for nested tuples of [`Table`]s like `(TableA, (TableB, (TableC, )))` and so on.
/// This theoretically allows for [`Schema`]s with any number (> 0) of [`Table`]s, though eventually you might compiler limits.
///
/// Again, don't bother implementing [`Schema`] manually, use [`#[database]`](crate::database).
pub trait TableList: private::Sealed {
	fn assemble_into(sql: &mut Vec<String>);
}

impl<T: Table> TableList for (T, ) {
	fn assemble_into(sql: &mut Vec<String>) {
		sql.push(T::assemble_sql())
	}
}
impl<T: Table, L: TableList> TableList for (T, L) {
	fn assemble_into(sql: &mut Vec<String>) {
		sql.push(T::assemble_sql());
		L::assemble_into(sql)
	}
}

mod private {
	use super::*;
	pub trait Sealed {}
	impl<T: Table> Sealed for (T, ) {}
	impl<T: Table, L: TableList> Sealed for (T, L) {}
}
