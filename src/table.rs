use rusqlite::Result as SqlResult;
use rusqlite::Row;

use crate::{
	Bind,
	Binder
};
use crate::column::{
	//Column,
	Definition
};
use crate::meta::tuple::{
	TupleRef,
	TupleAsRef,
	TupleAsMut
};
use crate::value::Value;

pub trait Table {
	const NAME: &'static str;
	// TODO: "ON CONFLICT " clause
	const PRIMARY_KEY: &'static [&'static str];
	const COLUMNS: &'static [&'static str];
	type References;
	type Values: ValueList;
	//TODO: CHECKS

	fn assemble_sql() -> String {
		let mut defs = Vec::new();
		Self::Values::assemble_into(&mut defs);

		let mut sql = String::from("CREATE TABLE ");
		sql += Self::NAME;
		sql += " (\n\t";
		let column_count = Self::COLUMNS.len();
		let column_iter = defs.iter()
			.rev()
			.zip(Self::COLUMNS)
			.enumerate()
			.map(|(idx, rest)| (idx + 1 < column_count, rest));
		for (not_last, (column, name)) in column_iter {
			column.write_sql_to(name, &mut sql);
			if not_last {
				sql.push_str(",\n\t");
			}
		}
		match Self::PRIMARY_KEY {
			//no primary key
			[] => {},
			//single or composite primary key
			[first, rest @ ..] => {
				sql.push_str(",\n\tPRIMARY KEY ( ");
				sql.push_str(first);
				for k in rest {
					sql.push_str(", ");
					sql.push_str(k);
				}
				sql.push_str(" )");
				// TODO: "ON CONFLICT " clause
			},
		}

		sql += "\n); ";
		sql
	}

	fn debug() {
		let ty = std::any::type_name::<Self>();
		let name = Self::NAME;
		println!("struct {ty} as table {name:?}:");
		println!("\tColumn Names: {:?}", Self::COLUMNS);
		let col_types = std::any::type_name::<Self::Values>();
		println!("\tColumn Types: {col_types}");
		println!("\tPrimary Key: {:?}", Self::PRIMARY_KEY);
		let refs = std::any::type_name::<Self::References>();
		println!("\tReferences: {refs}");

	}
}


pub trait Entry: Sized {
	const GET_ALL: &'static str;
	const INSERT: &'static str;

	fn bind_to(&self, binder: &mut Binder<'_>) -> SqlResult<()>;
	fn from_row(row: &Row) -> SqlResult<Self>;
}

pub trait HasKey {
	const GET_BY_KEY: &'static str;
	type Key: Bind + for<'t> TupleRef<'t>;

	fn get_key(&self) -> TupleAsRef<Self::Key>;
	fn get_key_mut(&mut self) -> TupleAsMut<Self::Key>;
}

pub trait ValueList {
	fn assemble_into(defs: &mut Vec<Definition>);
}


impl<V: Value> ValueList for (V, ) {
	fn assemble_into(defs: &mut Vec<Definition>) {
		defs.push(Definition::from_value::<V>("name"))
	}
}
impl<V: Value, L: ValueList> ValueList for (V, L) {
	fn assemble_into(defs: &mut Vec<Definition>) {
		defs.push(Definition::from_value::<V>("name"));
		L::assemble_into(defs)
	}
}
