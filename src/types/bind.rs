use rusqlite::{
	Statement,
	ToSql,
	Result as SqlResult,
};

pub trait Bind {
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()>;
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
	pub fn bind<T: Bind>(&mut self, thing: &T) -> SqlResult<()> {
		thing.bind(self)
	}
}

impl<T: ToSql + ToSql2> Bind for T {
	fn bind(&self, binder: &mut Binder<'_, '_>) -> SqlResult<()> {
		binder.bind_parameter(&self)?;
		Ok(())
	}
}

#[liter_derive::impl_tuple(2..=16)]
impl Bind for Each!(T) where Every!(T => T: Bind): '_ {
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
impl<T: ToSql> ToSql2 for Option<T> {}

impl ToSql2 for rusqlite::types::Null {}
impl ToSql2 for rusqlite::types::Value {}

