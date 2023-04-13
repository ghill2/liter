use liter_derive::impl_tuple;

use crate::bind::{
	Bind,
	ToSql2
};

pub trait TupleRef<'l> {
	type Ref;
	type Mut;
}

pub type TupleAsRef<'t, T> = <T as TupleRef<'t>>::Ref;
pub type TupleAsMut<'t, T> = <T as TupleRef<'t>>::Mut;

impl<'l, T: rusqlite::ToSql + ToSql2 + 'l> TupleRef<'l> for T {
	type Ref = &'l T;
	type Mut = &'l mut T;
}

impl_tuple! {
	2..=16:
	impl<'l> TupleRef<'l> for Sized where Self: 'l + Bind {
		type Ref = Each!(T => &'l T);
		type Mut = Each!(T => &'l mut T);
	}
}

