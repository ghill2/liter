use rusqlite::{
	Row,
	types::FromSql,
	Result as SqlResult,
};

pub trait Fetch: Sized {
	fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self>;

	fn from_row(row: Row) -> SqlResult<Self> {
		let mut fetcher = Fetcher::make(row);
		Self::fetch(&mut fetcher)
	}
}
pub trait FromSql2 {}

pub struct Fetcher<'stmt> {
	index: usize,
	row: Row<'stmt>
}

impl<'stmt> Fetcher<'stmt> {
	pub(crate) fn make(row: Row<'stmt>) -> Self {
		Self {index: 0, row}
	}
	#[inline]
	pub fn fetch<T: FromSql>(&mut self) -> SqlResult<T> {
		let thing = self.row.get(self.index)?;
		self.index += 1; // fetch parameter index is 0-based
		Ok(thing)
	}
	pub(crate) fn revert(self) -> Row<'stmt> {
		self.row
	}
}

liter_derive::impl_tuple!{
	1..=16:
	impl Fetch for FromSql + FromSql2 {
		fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self> {
			let fetched = each!{ fetcher.fetch()? };
			Ok(fetched)
		}
	}
}

impl FromSql2 for bool {}

impl FromSql2 for i8 {}
impl FromSql2 for i16 {}
impl FromSql2 for i32 {}
impl FromSql2 for i64 {}
impl FromSql2 for isize {}

impl FromSql2 for u8 {}
impl FromSql2 for u16 {}
impl FromSql2 for u32 {}
impl FromSql2 for u64 {}
impl FromSql2 for usize {}

impl FromSql2 for f32 {}
impl FromSql2 for f64 {}

impl FromSql2 for String {}
impl FromSql2 for Box<str> {}
impl FromSql2 for std::rc::Rc<str> {}
impl FromSql2 for std::sync::Arc<str> {}

impl FromSql2 for Vec<u8> {}
impl<const N: usize> FromSql2 for [u8; N] {}
impl<T: FromSql> FromSql2 for Option<T> {}

impl FromSql2 for rusqlite::types::Value {}

