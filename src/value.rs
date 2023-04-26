use rusqlite::Result as SqlResult;
use rusqlite::types::{
	FromSql,
	ToSql,
	ValueRef,
	FromSqlResult,
	ToSqlOutput
};

use crate::table::{
	HasKey,
	Table
};

pub trait Value: FromSql + ToSql {
	const AFFINITY: Affinity;
	const NULLABLE: bool = false;

	const UNIQUE: bool = false;
	const FOREIGN_KEY: Option<ForeignKey> = None;

	const CHECKS: &'static [Check] = &[];

	type References;
}

#[derive(Clone, Copy, Debug)]
pub enum Affinity {
	Integer,
	Real,
	Text,
	Blob,
}

#[derive(Clone, Copy, Debug)]
pub enum Check {
	// SQL string that will be prepended with the name of the column
	Sql(&'static str)
}

// Note: The Value does not know the Type that is being referenced
pub struct ForeignKey {
	pub table_name: &'static str,
	pub deferrable: bool,
	pub on_delete: FkConflictAction,
	pub on_update: FkConflictAction
}

#[derive(Clone, Copy, Debug)]
pub enum FkConflictAction {
	Cascade,
	Restrict,
	SetNull
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Id(Option<u64>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ref<T: HasKey>(pub T::Key);

impl<T: HasKey<Key = K>, K: Clone> Ref<T> {
	pub fn make_ref(from: &T) -> Self {
		Self(from.clone_key())
	}
}

/*
 *	DEFINITION ASSEMBLY
 */

impl Affinity {
	pub const fn as_str(self) -> &'static str {
		match self {
			Affinity::Integer => "INTEGER NOT NULL",
			Affinity::Real => "REAL NOT NULL",
			Affinity::Text => "TEXT NOT NULL",
			Affinity::Blob => "BLOB NOT NULL",
		}
	}
	pub const fn as_str_nullable(self) -> &'static str {
		match self {
			Affinity::Integer => "INTEGER",
			Affinity::Real => "REAL",
			Affinity::Text => "TEXT",
			Affinity::Blob => "BLOB",
		}
	}

}

impl ForeignKey {
	pub(crate) fn write_sql_to(&self, sql: &mut String) {
		sql.push_str(" REFERENCES ");
		sql.push_str(self.table_name);
		sql.push_str(" ON UPDATE ");
		sql.push_str(self.on_update.as_sql());
		sql.push_str(" ON DELETE ");
		sql.push_str(self.on_delete.as_sql());
		if self.deferrable {
			sql.push_str(" DEFERRABLE INITIALLY DEFERRED");
		}
	}
}


impl FkConflictAction {
	fn as_sql(self) -> &'static str {
		match self {
			Self::Cascade => "CASCADE",
			Self::Restrict => "RESTRICT",
			Self::SetNull => "SET NULL",
		}
	}
}

/*
 *	VALUES
 */

macro_rules! value {
	($t:ty, $col:expr) => {
		impl Value for $t {
			const AFFINITY: Affinity = $col;
			type References = ();
		}
	};
}

/* BLOB */
value!(Vec<u8>, Affinity::Blob);
impl<const N: usize> Value for [u8; N] {
	const AFFINITY: Affinity = Affinity::Blob;
	type References = ();
}

/* TEXT */
value!(std::rc::Rc<str>, Affinity::Text);
value!(std::sync::Arc<str>, Affinity::Text);
value!(Box<str>, Affinity::Text);
value!(String, Affinity::Text);

/* REAL */
value!(f32, Affinity::Real);
value!(f64, Affinity::Real);

/* INTEGER */
value!(i8, Affinity::Integer);
value!(i16, Affinity::Integer);
value!(i32, Affinity::Integer);
value!(i64, Affinity::Integer);

value!(u8, Affinity::Integer);
value!(u16, Affinity::Integer);
value!(u32, Affinity::Integer);
value!(u64, Affinity::Integer);

value!(usize, Affinity::Integer);

/* NULLABLE */
impl<T: Value> Value for Option<T> {
	const AFFINITY: Affinity = T::AFFINITY;
	const NULLABLE: bool = true;
	type References = T::References;
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

impl Value for Id {
	const AFFINITY: Affinity = Affinity::Integer;
	type References = ();
}

/* REFERENCE */

impl<T: HasKey<Key = Id>> Ref<T> {
	pub const NULL: Self = Self(Id::NULL);
}

impl<T: HasKey<Key = K>, K: FromSql> FromSql for Ref<T> {
	fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
		K::column_result(value).map(|key| Self(key))
	}
}
impl<T: HasKey<Key = K>, K: ToSql + 'static> ToSql for Ref<T> {
	fn to_sql(&self) -> SqlResult<ToSqlOutput<'_>> {
		self.0.to_sql()
	}
}
impl<T: HasKey> crate::bind::ToSql2 for Ref<T> {}

impl<T: Table + HasKey<Key = K>, K: FromSql + ToSql + 'static> Value for Ref<T> {
	const AFFINITY: Affinity = Affinity::Integer;
	const FOREIGN_KEY: Option<ForeignKey> = Some(ForeignKey {
		table_name: T::NAME,
		deferrable: true,
		on_delete: FkConflictAction::Restrict,
		on_update: FkConflictAction::Restrict
	});
	type References = T;
}

