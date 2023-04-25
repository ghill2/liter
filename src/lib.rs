pub mod bind;
pub use bind::{
	Bind,
	Binder
};
pub mod column;
pub mod meta;
pub mod schema;
pub use schema::Schema;
pub mod table;
pub use table::{
	Entry,
	HasKey,
	Table
};
pub mod value;
pub use value::{
	Value,
	Ref,
	Id
};

pub use liter_derive::{
	database,
	Table
};

use std::marker::PhantomData;
use std::path::Path;

use rusqlite::{
	Connection,
	Error,
	Result as SqlResult,
};

pub struct Database<S: Schema> {
	connection: Connection,
	schema: PhantomData<S>
}

impl<S: Schema> Database<S> {
	fn from_connection(connection: Connection) -> Self {
		Self { connection, schema: PhantomData }
	}
	pub fn open(path: &Path) -> SqlResult<Self> {
		Connection::open(path).map(Self::from_connection)
	}
	pub fn open_in_memory() -> SqlResult<Self> {
		Connection::open_in_memory().map(Self::from_connection)
	}

	pub fn create_in_memory() -> SqlResult<Self> {
		let new = Self::open_in_memory()?;
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
		let stmt = self.connection.prepare(T::GET_BY_KEY)?;
		let mut binder = Binder::make(stmt);
		key.bind(&mut binder)?;
		let mut stmt = binder.revert();
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
		let stmt = self.connection.prepare(T::INSERT)?;
		let mut binder = Binder::make(stmt);
		entry.bind_to(&mut binder)?;
		let mut stmt = binder.revert();
		let changes = stmt.raw_execute()?;
		if changes != 1 {
			return Err(Error::StatementChangedRows(changes));
		}
		let id = self.connection.last_insert_rowid();
		*entry.get_key_mut() = Id::from_u64(id as u64);
		Ok(())
	}

	pub fn insert<T: Entry>(&self, entry: &T) -> SqlResult<usize> {
		let stmt = self.connection.prepare(T::INSERT)?;
		let mut binder = Binder::make(stmt);
		entry.bind_to(&mut binder)?;
		let mut stmt = binder.revert();
		stmt.raw_execute()
	}

}
