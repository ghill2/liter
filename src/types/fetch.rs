use rusqlite::{
	Row,
	Result as SqlResult,
	Error
};
use rusqlite::types::{
	ValueRef,
	FromSql,
	Type
};

pub trait Fetch: Sized {
	fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self>;
	fn try_fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Option<Self>>;

	fn from_row(row: &Row) -> SqlResult<Self> {
		let mut fetcher = Fetcher::make(row);
		Self::fetch(&mut fetcher)
	}
}
pub trait FromSql2 {}

pub struct Fetcher<'row> {
	index: usize,
	all_nulls: bool,
	row: &'row Row<'row>
}

impl<'row> Fetcher<'row> {
	pub(crate) fn make(row: &'row Row<'row>) -> Self {
		Self {index: 0, all_nulls: true, row}
	}
	pub(crate) fn reset_all_nulls(&mut self) {
		self.all_nulls = true;
	}
	// TODO: this allows fetching an Option<T> without updating all_nulls
	#[inline]
	pub fn fetch_column<T: FromSql>(&mut self) -> SqlResult<T> {
		let thing = self.row.get(self.index)?;
		self.index += 1; // fetch parameter index is 0-based
		Ok(thing)
	}
	#[inline]
	pub fn try_fetch_column<T: FromSql>(&mut self) -> SqlResult<Option<T>> {
		let thing = if self.row.get_ref(self.index)? != ValueRef::Null {
			self.all_nulls = false;
			Some(self.row.get(self.index)?)
		}
		else {None};
		self.index += 1; // fetch parameter index is 0-based
		Ok(thing)
	}
	#[inline]
	#[must_use = "advances the column index"]
	pub fn borrow_column(&mut self) -> SqlResult<ValueRef> {
		let value_ref = self.row.get_ref(self.index)?;
		self.index += 1; // fetch parameter index is 0-based
		Ok(value_ref)
	}
	pub fn fetch<T: Fetch>(&mut self) -> SqlResult<T> {
		T::fetch(self)
	}
	pub fn try_fetch<T: Fetch>(&mut self) -> SqlResult<Option<T>> {
		T::try_fetch(self)
	}
	pub fn fetch_none<T: Fetch>(&mut self) -> SqlResult<()> {
		self.reset_all_nulls();
		// we can't just check whether this is `None` because `try_fetch`ing a tuple `(Option<A>, Option<B>)` from `NULL`s gives us `Some(None, None)` (and it has to do that, to make `impl Value for Option<V: Value>` work)
		let _ = T::try_fetch(self)?;
		// instead, we check whether during this `try_fetch`, any *columns* were non-null, which is technically not the same ("Option Collapse")
		//TODO: dodgy API boundary, since the user is supposed to implement `try_fetch` and yet here we are relying on its side effects w.r.t. `all_nulls`
		if self.all_nulls {Ok(())}
		else {Err(Error::FromSqlConversionFailure(
			self.index, //TODO wrong index
			Type::Null,
			"encountered non-NULL column expecting to fetch None".into()
		))}

	}
}

impl<T: FromSql + FromSql2> Fetch for T {
	fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self> {
		fetcher.fetch_column()
	}
	fn try_fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Option<Self>> {
		fetcher.try_fetch_column()
	}
}

impl<T: Fetch> Fetch for Option<T> {
	fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self> {
		T::try_fetch(fetcher)
	}
	fn try_fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Option<Self>> {
		// we need to return `Some(None)` instead of just `None` because the tuple `try_fetch` cannot check whether the generic type it is `try_fetch`ing is itself an `Option` already, i.e. it can only look at the outer `Option`.
		// Because of this, `None` for `try_fetch` doesn't just mean the column/columns *weren't* fetched, but that they *couldn't* be fetched -- and for `Option<T>` that is never the case, since it can be `Some(None)`
		T::try_fetch(fetcher).map(Some)
	}
}

#[liter_derive::impl_tuple(2..=40)]
impl Fetch for Each!(T) where Every!(T => T: Fetch): '_ {
	fn fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Self> {
		let fetched = each!{ fetcher.fetch()? };
		Ok(fetched)
	}
	fn try_fetch(fetcher: &mut Fetcher<'_>) -> SqlResult<Option<Self>> {
		fetcher.reset_all_nulls();
		// fetch all columns first to determine whether they are all nulls
		let fetched = each!{{
			let idx = fetcher.index;
			let all_nulls_before = fetcher.all_nulls;
			// this resets `all_nulls` so we carry it over manually
			let thing = fetcher.try_fetch()?;
			fetcher.all_nulls &= all_nulls_before;
			(idx, thing)
		}};
		let unwrapped = each!{
			@fetched,
			(idx, thing) => match thing {
				Some(t) => t,
				None if fetcher.all_nulls => return Ok(None),
				None => return Err(Error::FromSqlConversionFailure(
					idx, //TODO: index could be wrong (?)
					Type::Null,
					"NULL column in optional value with non-NULL columns".into()
				))
			}
		};
		Ok(Some(unwrapped))
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

impl FromSql2 for rusqlite::types::Value {}

