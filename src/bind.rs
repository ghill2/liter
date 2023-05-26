use rusqlite::{
	Statement,
	ToSql,
	Result as SqlResult,
};

pub trait Bind {
	fn bind(self, binder: &mut Binder<'_>) -> SqlResult<()>;
}
pub trait ToSql2 {}

pub struct Binder<'conn> {
	index: usize,
	stmt: Statement<'conn>
}

impl<'conn> Binder<'conn> {
	pub(crate) fn make(stmt: Statement<'conn>) -> Self {
		Self {index: 0, stmt}
	}
	#[inline]
	pub fn bind<T: ToSql>(&mut self, thing: T) -> SqlResult<()> {
		self.index += 1; // bind parameter index is 1-based
		self.stmt.raw_bind_parameter(self.index, thing)
	}
	pub(crate) fn revert(self) -> Statement<'conn> {
		self.stmt
	}
}

liter_derive::impl_tuple!{
	1..=16:
	impl Bind for ToSql + ToSql2 {
		fn bind(self, binder: &mut Binder<'_>) -> SqlResult<()> {
			each!{thing => {binder.bind(thing)?;} }
			Ok(())
		}
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

