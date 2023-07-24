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
	InnerValueDef
};

pub trait Table {
	const NAME: &'static str;
	const DEFINITION: TableDef;
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
}


