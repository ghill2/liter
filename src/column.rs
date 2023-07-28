use rusqlite::types::{
	FromSql,
	ToSql
};

use crate::value::Check;
use crate::fetch::FromSql2;
use crate::bind::ToSql2;

pub trait Column: FromSql + ToSql + FromSql2 + ToSql2 {
	const AFFINITY: Affinity;
	const NULLABLE: bool = false;
	const CHECKS: &'static [Check] = &[];

	const DEFINITION: ColumnDef = ColumnDef {
		affinity: Self::AFFINITY,
		nullable: Self::NULLABLE,
		checks: Self::CHECKS
	};
}

#[derive(Debug)]
pub struct ColumnDef {
	pub affinity: Affinity,
	pub nullable: bool,
	pub checks: &'static [Check]
}

#[derive(Clone, Copy, Debug)]
pub enum Affinity {
	Integer,
	Real,
	Text,
	Blob,
}

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

impl ColumnDef {
	/// Write out the [`Column`] SQL defintion
	///
	/// ```sql
	/// column_name INTEGER NOT NULL
	/// ```
	pub(crate) fn write_sql_to(&self, name: &str, sql: &mut String) {
		sql.push_str(name);
		sql.push(' ');
		if !self.nullable {
			sql.push_str(self.affinity.as_str());
		}
		else {
			sql.push_str(self.affinity.as_str_nullable());
		}
		for Check::Sql(check) in self.checks.iter() {
			sql.push_str(" CHECK ( ");
			sql.push_str(name);
			sql.push(' ');
			sql.push_str(check);
			sql.push_str(" ) ");
		}
	}
}

/*
 *	COLUMNS
 */

macro_rules! column {
	($t:ty, $col:expr) => {
		impl Column for $t {
			const AFFINITY: Affinity = $col;
		}
	};
}

/* BLOB */
column!(Vec<u8>, Affinity::Blob);
impl<const N: usize> Column for [u8; N] {
	const AFFINITY: Affinity = Affinity::Blob;
}

/* TEXT */
column!(std::rc::Rc<str>, Affinity::Text);
column!(std::sync::Arc<str>, Affinity::Text);
column!(Box<str>, Affinity::Text);
column!(String, Affinity::Text);

/* REAL */
column!(f32, Affinity::Real);
column!(f64, Affinity::Real);

/* INTEGER */
column!(i8, Affinity::Integer);
column!(i16, Affinity::Integer);
column!(i32, Affinity::Integer);
column!(i64, Affinity::Integer);

column!(u8, Affinity::Integer);
column!(u16, Affinity::Integer);
column!(u32, Affinity::Integer);
column!(u64, Affinity::Integer);

column!(usize, Affinity::Integer);

/* NULLABLE */
impl<T: Column> Column for Option<T> {
	const AFFINITY: Affinity = T::AFFINITY;
	const NULLABLE: bool = true;
}

