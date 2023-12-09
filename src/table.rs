//! SQL [`Table`]s defined at compile-time

use construe::{
	Construe,
	StrConstrue,
	write
};

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
	NestedValueDef,
	StrChain
};

/// SQL table that can be used in a [`database`](crate::database)
///
/// The items that make up this trait are mostly an implementation detail.
/// See the [`Entry`] trait (which is also implemented by the `#[derive(Table)]` proc-macro) as well as the [`HasKey`] trait (same, but only if the table has a primary key) for SQL generated to be used by you.
pub trait Table {
	/// Name of the table: `#[derive(Table)]` uses the lowercase name of the struct
	const NAME: &'static str;
	/// The [`TableDef`] struct defines the table and is used to assemble the `CREATE_TABLE` SQL statement
	const DEFINITION: TableDef;
	/// `CREATE TABLE` SQL statement
	const CREATE_TABLE: &'static str;

	/// Names of all the [`Column`](crate::Column)s that make up the table.
	const ALL_COLUMNS: &'static [&'static str];
	/// Names of the [`Column`](crate::Column)s that make up the values of the primary `#[key]`
	///
	/// This is empty if the [`Table`] has no primary key.
	const KEY_COLUMNS: &'static [&'static str];
	/// Names of all the [`Column`](crate::Column)s that make up the table, which *aren't* part of the primary key.
	const OTHER_COLUMNS: &'static [&'static str];

	/// Nested tuple that contains all the types (i.e. other tables) referenced (e.g. via [`Ref`]) by [`Value`](crate::Value)s in this table.
	///
	/// See the [`meta`](crate::meta) module for a little more detail on how this is used for [`Schema`](crate::schema) validation.
	type References;
}

/// `'static` array of name & [`ValueDef`] pairs
pub type Values = &'static [(&'static str, ValueDef)];

/// Definition of a [`Table`] used to generate the SQL schema
///
/// Created by the `#[derive(Table)]` proc-macro.
#[derive(Debug)]
pub struct TableDef {
	// TODO: "ON CONFLICT " clause
	//on_conflict: ???,
	/// Name of the table: `#[derive(Table)]` uses the lowercase name of the struct
	pub name: &'static str,
	/// Names of the [`Column`](crate::Column)s that make up the values of the primary `#[key]`
	///
	/// This is empty if the [`Table`] has no primary key.
	pub primary_key:  &'static [&'static str],
	/// Definitions and names for all constituent [`Value`](crate::Value)s
	pub values: Values,
	/// Definitions and names for all (if any) primary key [`Value`](crate::Value)s
	pub key_values: Values,
	/// Definitions and names for all [`Value`](crate::Value)s that are *not* (part of) the primary key.
	///
	/// This could be empty (if all [`Value']s are part of the primary key), or it could be equal to `values`.
	pub other_values: Values,
	/// List of [`Table`]-level [`Constraint`]s
	pub constraints: &'static [Constraint],
}

/// SQL constraint at the [`Table`]-level
#[derive(Debug)]
pub enum Constraint {
	/// SQL that will be put into a `CHECK (…)` constraint unmodified
	SqlCheck(&'static str),
	/// `UNIQUE` constraint over all the [`Values`]' columns
	Unique(Values)
}

/// SQL statements for interacting with a [`Table`]
///
/// This trait is also implemented by `#[derive(Table)]` and contains SQL statements to retrieve and insert entries from/into the table.
pub trait Entry: Sized + Fetch + Bind {
	/// `SELECT (...) FROM ...`
	///
	/// Select all [`Column`](crate::Column)s of all rows in this [`Table`].
	/// [`Fetch::from_row`] can be used to convert the results of the query to this type.
	const GET_ALL: &'static str;
	/// `INSERT INTO ... VALUES ( ?, ...)`
	///
	/// Insert a new row into the [`Table`] for this type.
	/// [`Bind`] is used to bind an instance of this type to the parameters in the correct order.
	const INSERT: &'static str;
}

/// [`Table`] that has a primary key, which may be composite
pub trait HasKey {
	/// `SELECT (...) WHERE (... = ?)`
	///
	/// Select an entry by its primary key.
	/// Binds however many columns the key has.
	const GET_BY_KEY: &'static str;
	/// `INSERT INTO ... VALUES ( ?, ...) ON CONFLICT DO UPDATE SET (x = excluded.x)`\*
	///
	/// \*: Instead of `DO UPDATE SET (…)` it's `DO NOTHING` for key-only tables.
	///
	/// Be aware that this doesn't actually specify the "conflict target", that is, on violation of which uniqueness constraint to `DO UPDATE SET`, it simply *assumes* it is because of the primary key, *and* it will only update the non-key columns to the `excluded` values.
	/// As such, you should probably not use this for tables with other `UNIQUE` constraints.  
	/// See <https://sqlite.org/lang_upsert.html> for more on the "upsert" statement, which is not standard SQL.
	const UPSERT: &'static str;
	/// `UPDATE (...) SET (... = ?) WHERE (... = ?)`
	///
	/// Update all the non-key values in the row `WHERE` the key matches.
	const UPDATE: &'static str;
	/// Delete a row by its primary key.
	const DELETE: &'static str;

	/// Definition of the key value (used by [`Ref`])
	const KEY_VALUE: NestedValueDef;
	/// Marker type used to disambiguate bounds on the key
	type Marker: Marker;
	/// Type of the key.
	/// If the key is composite, this will be a tuple.
	type Key: Fetch + Bind + Tuple<Self::Marker>;

	fn get_key(&self) -> <Self::Key as Tuple<Self::Marker>>::Ref<'_>;
	fn get_key_mut(&mut self) -> <Self::Key as Tuple<Self::Marker>>::Mut<'_>;

	fn make_ref(&self) -> Ref<Self>
		where Self::Key: CloneFromRef<Self::Marker>
	{
		Ref(Self::Key::clone_from_ref(self.get_key()))
	}
}

/// Alias for [`HasKey`] restricted to non-composite primary keys
pub trait HasSingleKey<K>: HasKey<Key = K, Marker = marker::One> {}
/// Alias for [`HasKey`] restricted to composite primary keys
pub trait HasCompositeKey<K>: HasKey<Key = K, Marker = marker::Many> {}

impl<T: HasKey<Marker = marker::One>> HasSingleKey<T::Key> for T {}
impl<T: HasKey<Marker = marker::Many>> HasCompositeKey<T::Key> for T {}

impl TableDef {
	pub const fn define<const N: usize>(&self) -> StrConstrue<N> {
		let mut sc = StrConstrue::new();
		sc = sc.push_str("CREATE TABLE ");
		sc = sc.push_str(self.name);
		sc = sc.push_str(" (\n\t");

		let [(first_name, first_def), other_values @ ..] = self.values else {
			panic!("empty table")
		};

		// DEFINE COLUMNS
		sc = first_def.push_sql(first_name, sc);
		let mut values = other_values;
		while let [(name, def), rest @ ..] = values {
			values = rest;
			sc = sc.push_str(",\n\t");
			sc = def.push_sql(name, sc);
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
		sc = first_def.push_constraint_sql(&StrChain::start(first_name), sc);
		let mut values = other_values;
		while let [(name, def), rest @ ..] = values {
			values = rest;
			sc = def.push_constraint_sql(&StrChain::start(name), sc);
		}

		// ADD TABLE-LEVEL CHECKS
		let mut constraints = self.constraints;
		while let [constraint, rest @ ..] = constraints {
			constraints = rest;
			sc = sc.push_str(",\n\t");
			sc = constraint.push_sql(sc);
		}

		sc.push_str("\n) STRICT;")
	}
}

impl Constraint {
	const fn push_sql<const N: usize>(&self, mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		match *self {
			Self::SqlCheck(sql) => sc.push_str("CHECK (")
				.push_str(sql)
				.push_str(")"),
			Self::Unique(mut values) => {
				sc = sc.push_str("UNIQUE (");
				while let [(name, value), rest @ ..] = values {
					values = rest;
					sc = value.inner.push_column_names(
						&StrChain::start(name),
						sc
					);
					if !rest.is_empty() {
						sc = sc.push_str(", ");
					}
				}

				sc.push_str(")")
			}
		}
	}
}

/// Generates the [`HasKey::GET_BY_KEY`] statement at compile-time
pub const fn get_by_key<const N: usize>(name: &str, key_columns: &[&str])
	-> StrConstrue<N>
{
	let mut sc = StrConstrue::new();
	sc = sc.push_str("SELECT * FROM ")
		.push_str(name)
		.push_str(" WHERE (");

	let [first, other_columns @ ..] = key_columns else {
		panic!("no key columns")
	};
	sc = sc.push_str(first).push_str(" = ?");

	let mut columns = other_columns;
	while let [name, rest @ ..] = columns {
		sc = sc.push_str(" AND ").push_str(name).push_str(" = ?");
		columns = rest;
	}

	sc.push_str(")")
}

/// Generates the [`HasKey::DELETE`] statement at compile-time
pub const fn delete<const N: usize>(name: &str, key_columns: &[&str])
	-> StrConstrue<N>
{
	let mut sc = StrConstrue::new();
	write!(sc, "DELETE FROM ", name, " WHERE (");

	let [first, other_columns @ ..] = key_columns else {
		panic!("no key columns")
	};
	write!(sc, *first, " = ?");

	let mut columns = other_columns;
	while let [name, rest @ ..] = columns {
		write!(sc, " AND ", *name, " = ?");
		columns = rest;
	}
	sc.push_str(")")
}

/// Generates the [`Entry::INSERT`] statement at compile-time
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

/// Generates the [`HasKey::UPSERT`] statement at compile-time
pub const fn upsert<const N: usize>(
	name: &str,
	key_columns: &[&str],
	other_columns: &[&str])
	-> StrConstrue<N>
{
	let mut sc = StrConstrue::new();
	sc = sc.push_str("INSERT INTO \"")
		.push_str(name)
		.push_str("\" VALUES (?");

	// start from 1 with the first ? already written to not have trailing comma
	let mut i = 1;
	while i < key_columns.len() + other_columns.len() {
		sc = sc.push_str(", ?");
		i += 1;
	}
	sc = sc.push_str(") ON CONFLICT (");

	let [first, other_key_columns @ ..] = key_columns else {
		panic!("no key columns")
	};
	sc = sc.push_str(first);

	let mut columns = other_key_columns;
	while let [name, rest @ ..] = columns {
		sc = sc.push_str(", ").push_str(name);
		columns = rest;
	}
	sc = sc.push_str(") ");

	let [first, other_non_key_columns @ ..] = other_columns else {
		// key-only table
		return sc.push_str("DO NOTHING");
	};
	sc = sc.push_str("DO UPDATE SET ")
		.push_str(first)
		.push_str(" = excluded.")
		.push_str(first);

	let mut columns = other_non_key_columns;
	while let [name, rest @ ..] = columns {
		sc = sc.push_str(", ")
			.push_str(name)
			.push_str(" = excluded.")
			.push_str(name);
		columns = rest;
	}

	sc
}

/// Generates the [`HasKey::UPDATE`] statement at compile-time
pub const fn update<const N: usize>(
	name: &str,
	val_is_key_and_count: &[(bool, usize)],
	all_columns: &[&str])
	-> StrConstrue<N>
{
	assert!(!all_columns.is_empty(), "table must have at least one column");

	// build UPDATE statement by iterating over val_is_key_and_count twice
	// - first pass writes all the non-key columns, second all the key columns
	// - param_idx is the SQL parameter index (?n) with the Entry Bind order

	let mut sc = StrConstrue::new();
	write!(sc, "UPDATE \"", name, "\" SET ");

	// SET col_a = ?1, col_c = ?3, col_d = ?4

	let mut is_first = true;
	let mut param_idx = 0;
	let mut val_idx = 0;
	while val_idx < val_is_key_and_count.len() {
		let (is_key, count) = val_is_key_and_count[val_idx];
		if is_key {
			param_idx += count;
		}
		else {
			let last_col_idx = param_idx + count;
			while param_idx < last_col_idx {
				if !is_first {
					sc = sc.push_str(", ");
				}
				else {is_first = false;}
				write!(
					sc,
					all_columns[param_idx],
					" = ?",
					param_idx + 1 // params 1-based
				);
				param_idx += 1;
			}
		}
		val_idx += 1;
	}
	sc = sc.push_str(" WHERE ");

	// WHERE col_b = ?2 AND col_e = ?5 AND col_f = ?6

	let mut is_first = true;
	let mut param_idx = 0;
	let mut val_idx = 0;
	while val_idx < val_is_key_and_count.len() {
		let (is_key, count) = val_is_key_and_count[val_idx];
		if !is_key {
			param_idx += count;
		}
		else {
			let last_col_idx = param_idx + count;
			while param_idx < last_col_idx {
				if !is_first {
					sc = sc.push_str(" AND ");
				}
				else {is_first = false;}
				write!(
					sc,
					all_columns[param_idx],
					" = ?",
					param_idx + 1 // params 1-based
				);
				param_idx += 1;
			}
		}
		val_idx += 1;
	}
	sc
}

/// Helper struct to generate a slice of [`Column`](crate::Column) names at compile-time
pub struct Names<const C: usize, const L: usize> {
	bytes: StrConstrue<L>,
	names: Construe<(usize, usize), C>
}
/// Helper struct that stores a buffer of [`Column`](crate::Column) names and an array of indices into it
pub struct NameArrays<const C: usize, const L: usize> {
	bytes: [u8; L],
	names: [(usize, usize); C]
}

impl Names<0, 0> {
	pub const fn calculate_lengths(values: Values) -> (usize, usize) {
		let new = Self::from_values(values);
		(new.names.len(), new.bytes.len())
	}
}
impl<const C: usize, const L: usize> Names<C, L> {
	const fn new() -> Self {
		Self {
			bytes: StrConstrue::new(),
			names: Construe::new()
		}
	}

	pub const fn from_values(mut values: Values) -> Self {
		let mut new = Self::new();
		while let [(val_name, val_def), rest @ ..] = values {
			values = rest;
			new = new.collect_columns(val_def, val_name);
		}
		new
	}
	const fn collect_columns(self, def: &ValueDef, name: &str) -> Self {
		let name_chain = StrChain::start(name);
		self.traverse_value(&name_chain, &def.inner)
	}
	const fn traverse_value(
		mut self,
		chain: &StrChain<'_>,
		def: &NestedValueDef)
		-> Self
	{
		match def {
			NestedValueDef::Column(_def) => {
				let start = self.bytes.len();
				self.bytes = chain.join(self.bytes, "_");
				let end = self.bytes.len();
				self.names = self.names.push((start, end)).0;
				self
			},
			NestedValueDef::Value(def) => self.traverse_value(chain, &def.inner),
			NestedValueDef::Values([(first_name, first_def), rest @ ..]) => {
				// this descends
				self = self.traverse_value(
					&chain.with(first_name),
					&first_def.inner
				);
				// this doesn't actually descend (yet), it's just unpacking
				self.traverse_value(chain, &NestedValueDef::Values(rest))
			},
			NestedValueDef::Values([]) => self
		}
	}
	pub const fn finish(self) -> NameArrays<C, L> {
		NameArrays {
			bytes: self.bytes.store_bytes(),
			names: self.names.finish()
		}
	}
	pub const fn slice_array(arrays: &NameArrays<C, L>) -> [&str; C] {
		let mut bytes = arrays.bytes.as_slice();
		let mut array = [""; C];
		let mut i = 0;
		while i < C {
			let (start, end) = arrays.names[i];
			let (name, rest) = bytes.split_at(end - start);
			array[i] = match std::str::from_utf8(name) {
				Ok(n) => n,
				Err(_e) => panic!("assembled byte slice contains invalid UTF-8")
			};
			bytes = rest;
			i += 1;
		}
		array
	}
}

#[doc(hidden)]
#[macro_export]
macro_rules! column_names {
	($name:ident => $values:expr) => {
		const $name: &'static [&'static str] = {
			use $crate::table::Names;
			const VALUES: $crate::table::Values = $values;
			const LENGTHS: (usize, usize) = Names::calculate_lengths(&VALUES);
			const ARRAYS: $crate::table::NameArrays<{LENGTHS.0}, {LENGTHS.1}> =
				Names::from_values(&VALUES).finish();
			const NAMES: [&str; LENGTHS.0] = Names::slice_array(&ARRAYS);
			&NAMES
		};
	};
	($def:expr) => {
		$crate::table::column_names!(ALL_COLUMNS => $def.values);
		$crate::table::column_names!(KEY_COLUMNS => $def.key_values);
		$crate::table::column_names!(OTHER_COLUMNS => $def.other_values);
	}
}

/// Used by `#[derive(Table)]` to generate column name slices at compile time
#[doc(inline)]
pub use column_names;

