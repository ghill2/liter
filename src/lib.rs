pub mod bind;
pub use bind::{
	Bind,
	Binder
};
pub mod column;
pub use column::Column;
pub mod fetch;
pub use fetch::Fetch;
pub mod meta;
pub mod schema;
pub use schema::Schema;
pub mod table;
pub use table::{
	Entry,
	HasKey,
	Table
};
pub mod util;
pub mod value;
pub use value::Value;

pub use liter_derive::{
	database,
	Table
};

use std::marker::PhantomData;
use std::path::Path;

use rusqlite::{
	Connection,
	Error,
	Result as SqlResult
};
use rusqlite::types::{
	FromSql,
	ToSql,
	ValueRef,
	FromSqlResult,
	ToSqlOutput
};

use crate::column::Affinity;
use crate::value::{
	ForeignKey,
	FkConflictAction,
	ValueDef
};


pub struct Database<S: Schema> {
	connection: Connection,
	schema: PhantomData<S>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Id(Option<u64>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ref<T: HasKey + ?Sized>(pub T::Key);

/* DATABASE */

impl<S: Schema> Database<S> {
	fn from_connection(connection: Connection) -> SqlResult<Self> {
		connection.pragma_update(None, "foreign_keys", "on")?;
		Ok(Self { connection, schema: PhantomData })
	}
	pub fn open(path: &Path) -> SqlResult<Self> {
		Connection::open(path).and_then(Self::from_connection)
	}
	pub fn create_in_memory() -> SqlResult<Self> {
		let new = Connection::open_in_memory().and_then(Self::from_connection)?;
		new.connection.execute_batch(&S::define())?;
		Ok(new)
	}

	pub fn debug_show(&self) -> SqlResult<()> {
		let mut q = self.connection.prepare("SELECT * FROM pragma_table_list")?;
		let mut rows = q.query([])?;
		println!("(schema, name, ty, ncol, wr, strict)");
		while let Some(row) = rows.next()? {
			let r: (String, String, String, u64, bool, bool) =
				row.try_into()?;
			let (schema, name, ty, ncol, wr, strict) = r;
			println!("{schema}, {name}, {ty}, {ncol}, {wr}, {strict}");
		}

		let mut q = self.connection.prepare("SELECT * FROM sqlite_schema")?;
		let mut rows = q.query([])?;
		println!();
		println!("Schema:");
		while let Some(row) = rows.next()? {
			let r: (String, String, String, u64, Option<String>) =
				row.try_into()?;
			let (ty, name, tbl_name, rootpage, sql) = r;
			match name == tbl_name {
				true => print!("{ty} {name}:  (@ {rootpage})"),
				false => print!("{ty} {name}:	(→ {tbl_name} | @ {rootpage})"),
			}
			match sql {
				Some(sql) => println!("\n{sql}"),
				None => println!("\t<no SQL>")
			}
		}
		println!();

		Ok(())

	}

	pub fn get_all<T: Entry>(&self) -> SqlResult<Vec<T>> {
		let mut stmt = self.connection.prepare(T::GET_ALL)?;
		let mut rows = stmt.query([])?;
		let mut entries = Vec::new();
		while let Some(row) = rows.next()? {
			entries.push(T::from_row(row)?);
		}
		Ok(entries)
	}

	pub fn get<T>(&self, key: <T as HasKey>::Key) -> SqlResult<Option<T>>
		where T: Entry + HasKey
	{
		let mut stmt = self.connection.prepare(T::GET_BY_KEY)?;
		Binder::make(&mut stmt).bind(&key)?;
		let mut rows = stmt.raw_query();
		rows.next()?
			.map(T::from_row)
			.transpose()
	}

	/// Special method to insert and set id to last_insert_rowid
	pub fn create<T>(&self, entry: &mut T) -> SqlResult<()>
		where T: Entry + HasKey<Key = Id>
	{
		if entry.get_key() != &Id::NULL {
			return Err(Error::ToSqlConversionFailure(format!(
				"tried to create entry that already had the ID {:?}",
				entry.get_key()
			).into()));
		}
		let mut stmt = self.connection.prepare(T::INSERT)?;
		Binder::make(&mut stmt).bind(&*entry)?;
		let changes = stmt.raw_execute()?;
		if changes != 1 {
			return Err(Error::StatementChangedRows(changes));
		}
		let id = self.connection.last_insert_rowid();
		*entry.get_key_mut() = Id::from_u64(id as u64);
		Ok(())
	}

	pub fn insert<T: Entry>(&self, entry: &T) -> SqlResult<usize> {
		let mut stmt = self.connection.prepare(T::INSERT)?;
		Binder::make(&mut stmt).bind(entry)?;
		stmt.raw_execute()
	}

	pub fn execute<T: Bind>(&self, sql: &str, params: &T) -> SqlResult<usize> {
		let mut stmt = self.prepare(sql)?;
		Binder::make(&mut stmt).bind(params)?;
		stmt.raw_execute()
	}
}

impl<S: Schema> std::ops::Deref for Database<S> {
	type Target = Connection;
	fn deref(&self) -> &Self::Target {&self.connection}
}
impl<S: Schema> std::ops::DerefMut for Database<S> {
	fn deref_mut(&mut self) -> &mut Self::Target {&mut self.connection}
}

/* ID */

impl Id {
	pub const NULL: Self = Self(None);
	pub(crate) fn from_u64(id: u64) -> Self {Self(Some(id))}
}

impl FromSql for Id {
	fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
		u64::column_result(value).map(Some).map(Self)
	}
}
impl ToSql for Id {
	fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
		self.0.to_sql()
	}
}
impl crate::bind::ToSql2 for Id {}
impl crate::fetch::FromSql2 for Id {}

impl Column for Id {
	const AFFINITY: Affinity = Affinity::Integer;
}

/* REFERENCE */

impl<T: HasKey<Key = Id>> Ref<T> {
	pub const NULL: Self = Self(Id::NULL);
}
impl<T: HasKey<Key = K>, K: Clone> Ref<T> {
	pub fn make_ref(from: &T) -> Self {
		Self(from.clone_key())
	}
}

impl<T: Table + HasKey> Value for Ref<T> {
	const DEFINITION: ValueDef = ValueDef {
		unique: false,
		inner: T::KEY_VALUE,
		reference: Some(ForeignKey {
			table_name: T::NAME,
			deferrable: true,
			on_delete: FkConflictAction::Restrict,
			on_update: FkConflictAction::Restrict
		}),
		checks: &[],
	};
	type References = T;
}

impl<T: Table + HasKey> Fetch for Ref<T> {
	fn fetch(fetcher: &mut fetch::Fetcher<'_>) -> SqlResult<Self> {
		T::Key::fetch(fetcher).map(Self)
	}
}
impl<T: Table + HasKey> Bind for Ref<T> {
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		self.0.bind(binder)
	}
}
impl<T: Table + HasKey> Bind for &Ref<T> {
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		self.0.bind(binder)
	}
}
