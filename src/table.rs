use construe::StrConstrue;

use crate::{
	Bind,
	Fetch,
	Ref
};
use crate::meta::tuple::{
	Tuple,
	marker,
	Marker,
	CloneFromRef
};
use crate::value::{
	ValueDef,
	InnerValueDef,
	StrChain
};

pub trait Table {
	const NAME: &'static str;
	const DEFINITION: TableDef;
	const CREATE_TABLE: &'static str;

	type References;
}

pub struct TableDef {
	// TODO: "ON CONFLICT " clause
	//on_conflict: ???,
	pub name: &'static str,
	pub primary_key:  &'static [&'static str],
	pub values: &'static [(&'static str, ValueDef)],
	pub checks: &'static [Check],
}

#[derive(Debug)]
pub struct Check {
	pub sql: &'static str
}

pub trait Entry: Sized + Fetch + Bind {
	const GET_ALL: &'static str;
	const INSERT: &'static str;
}

pub trait HasKey {
	const GET_BY_KEY: &'static str;
	const KEY_VALUE: InnerValueDef;
	type Marker: Marker;
	type Key: Fetch + Bind + Tuple<Self::Marker>;

	fn get_key(&self) -> <Self::Key as Tuple<Self::Marker>>::Ref<'_>;
	fn get_key_mut(&mut self) -> <Self::Key as Tuple<Self::Marker>>::Mut<'_>;

	fn make_ref(&self) -> Ref<Self>
		where Self::Key: CloneFromRef<Self::Marker>
	{
		Ref(Self::Key::clone_from_ref(self.get_key()))
	}
}

pub trait HasSingleKey<K>: HasKey<Key = K, Marker = marker::One> {}
pub trait HasCompositeKey<K>: HasKey<Key = K, Marker = marker::Many> {}

impl<T: HasKey<Marker = marker::One>> HasSingleKey<T::Key> for T {}
impl<T: HasKey<Marker = marker::Many>> HasCompositeKey<T::Key> for T {}

impl TableDef {
	pub fn write_sql(&self) -> String {

		let mut sql = String::from("CREATE TABLE ");
		sql += self.name;
		sql += " (\n\t";

		let mut table_constraints = String::new();

		let [(first_name, first_def), rest @ ..] = self.values else {
			unreachable!("empty table")
		};
		first_def.define(first_name, &mut sql, &mut table_constraints);
		for (name, def) in rest {
			sql.push_str(",\n\t");
			def.define(name, &mut sql, &mut table_constraints);
		}

		match self.primary_key {
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

		sql += &table_constraints;

		sql += "\n); ";
		sql
	}

	pub const fn define<const N: usize>(&self) -> StrConstrue<N> {
		let mut sc = StrConstrue::new();
		sc = sc.push_str("CREATE TABLE ");
		sc = sc.push_str(self.name);
		sc = sc.push_str(" (\n\t");

		let [(first_name, first_def), other_values @ ..] = self.values else {
			panic!("empty table")
		};

		// DEFINE COLUMNS
		sc = first_def.inner.push_sql(&StrChain::start(first_name), sc);
		let mut values = other_values;
		while let [(name, def), rest @ ..] = values {
			values = rest;
			sc = sc.push_str(",\n\t");
			sc = def.inner.push_sql(&StrChain::start(name), sc);
		}

		// DEFINE PRIMARY KEY CONSTRAINT
		match self.key_values {
			//no primary key
			[] => {},
			//single or composite primary key
			[(first_name, first_def), rest @ ..] => {
				sc = sc.push_str(",\n\tPRIMARY KEY ( ");
				sc = first_def.inner
					.push_column_names(&StrChain::start(first_name), sc);
				let mut key_values = rest;
				while let [(name, def), rest @ ..] = key_values {
					key_values = rest;
					sc = sc.push_str(", ");
					sc = def.inner
						.push_column_names(&StrChain::start(name), sc);
				}
				sc = sc.push_str(" )");
				// TODO: "ON CONFLICT " clause
			}
		}

		// DEFINE VALUE CONSTRAINTS AT TABLE LEVEL
		sc = first_def.push_constraint_sql(first_name, sc);
		let mut values = other_values;
		while let [(name, def), rest @ ..] = values {
			values = rest;
			sc = def.push_constraint_sql(name, sc);
		}

		// ADD TABLE-LEVEL CHECKS
		let mut checks = self.checks;
		while let [check, rest @ ..] = checks {
			checks = rest;
			sc = sc.push_str(",\n\tCHECK (")
				.push_str(check.sql)
				.push_str(")");
		}

		sc.push_str("\n) STRICT;")
	}
}


pub const fn insert<const N: usize>(name: &str, column_count: usize)
	-> StrConstrue<N>
{
	assert!(column_count >= 1, "table must have at least one column");

	let mut sc = StrConstrue::new();
	sc = sc.push_str("INSERT INTO \"")
		.push_str(name)
		.push_str("\" VALUES (?");

	// start from 1 with the first ? already written to not have trailing comma
	let mut i = 1;
	while i < column_count {
		sc = sc.push_str(", ?");
		i += 1;
	}
	sc.push_str(")")
}

