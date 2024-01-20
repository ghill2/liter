use rusqlite::{
	Statement,
	ToSql,
	Result as SqlResult,
};

pub trait Bind {
	const COLUMNS: usize;
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()>;

	/// Bind to the [`Statement`]'s parameters **starting from the beginning**
	///
	/// This method will always bind to the parameters starting from the first parameter, even if those parameters were already bound to by previous call of `bind_to`!
	/// So, unless you want to overwrite previously bound parameters, do not call this function multiple times on the same [`Statement`].
	/// For instance, if you want to bind `"Hello"` and `123` to a statement, *don't* do:
	/// ```
	/// # use liter::Bind;
	/// # fn x(stmt: &mut rusqlite::Statement) -> rusqlite::Result<()> {
	/// "Hello".bind_to(stmt)?;
	/// 123.bind_to(stmt)?; // Wrong: this overwrites the "Hello"
	/// # Ok(())
	/// # }
	/// ```
	/// Instead, call it once with a tuple like so:
	/// ```
	/// # use liter::Bind;
	/// # fn x(stmt: &mut rusqlite::Statement) -> rusqlite::Result<()> {
	/// ("Hello", 123).bind_to(stmt)?; // Correct: binds both "Hello" and 123
	/// # Ok(())
	/// # }
	/// ```
	fn bind_to(&self, stmt: &mut Statement) -> SqlResult<()> {
		let mut binder = Binder::make(stmt);
		self.bind(&mut binder)
	}
}
pub trait ToSql2 {}

pub struct Binder<'stmt, 'conn> {
	index: usize,
	stmt: &'stmt mut Statement<'conn>
}

impl<'stmt, 'conn> Binder<'stmt, 'conn> {
	pub(crate) fn make(stmt: &'stmt mut Statement<'conn>) -> Self {
		Self {index: 0, stmt}
	}
	#[inline]
	pub fn bind_parameter<T: ToSql>(&mut self, thing: &T) -> SqlResult<()> {
		self.index += 1; // bind parameter index is 1-based
		self.stmt.raw_bind_parameter(self.index, thing)
	}
	pub fn bind<T: Bind + ?Sized>(&mut self, thing: &T) -> SqlResult<()> {
		thing.bind(self)
	}
	pub fn skip(&mut self, count: usize) {
		self.index += count;
	}
}

impl<T: ToSql + ToSql2 + ?Sized> Bind for T {
	const COLUMNS: usize = 1;
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		binder.bind_parameter(&self)?;
		Ok(())
	}
}

impl<T: Bind> Bind for Option<T> {
	const COLUMNS: usize = T::COLUMNS;
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		if let Some(slf) = self {
			binder.bind(slf)?;
		}
		else {
			// "Parameters that are not assigned values using sqlite3_bind() are treated as NULL."
			// See https://sqlite.org/lang_expr.html#varparam
			binder.skip(T::COLUMNS);
		}
		Ok(())
	}
}

#[liter_derive::impl_tuple(2..=16)]
impl Bind for Each!(T) where Every!(T => T: Bind): '_ {
	const COLUMNS: usize = {
		let mut columns = 0;
		Each!(T => columns += T::COLUMNS);
		columns
	};
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		each!{ref thing => {binder.bind(thing)?;} };
		Ok(())
	}
}


impl ToSql2 for bool {}

impl ToSql2 for i8 {}
impl ToSql2 for i16 {}
impl ToSql2 for i32 {}
impl ToSql2 for i64 {}
impl ToSql2 for isize {}

impl ToSql2 for u8 {}
impl ToSql2 for u16 {}
impl ToSql2 for u32 {}
impl ToSql2 for u64 {}
impl ToSql2 for usize {}

impl ToSql2 for f32 {}
impl ToSql2 for f64 {}

impl<T: ToSql + ?Sized> ToSql2 for std::rc::Rc<T> {}
impl<T: ToSql + ?Sized> ToSql2 for std::sync::Arc<T> {}
impl<T: ToSql + ?Sized> ToSql2 for Box<T> {}

impl ToSql2 for String {}
impl ToSql2 for str {}

impl ToSql2 for Vec<u8> {}
impl ToSql2 for [u8] {}

impl<T: ?Sized + ToSql> ToSql2 for &'_ T {}
impl<const N: usize> ToSql2 for [u8; N] {}

impl ToSql2 for rusqlite::types::Null {}
impl ToSql2 for rusqlite::types::Value {}

