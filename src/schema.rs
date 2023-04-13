use crate::Table;

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

pub trait TableList {
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
