use liter_derive::impl_tuple;

pub use marker::Marker;

pub mod marker {
	pub struct One;
	pub struct Many;

	pub trait Marker: private::Sealed {}
	impl Marker for One {}
	impl Marker for Many {}
	mod private {
		pub trait Sealed {}
		impl Sealed for super::One {}
		impl Sealed for super::Many {}
	}
}

pub trait Tuple<M: Marker>: private::Sealed<M> {
	type Ref<'l> where Self: 'l;
	type Mut<'l> where Self: 'l;
	fn take_ref(&self) -> Self::Ref<'_>;
}

mod private {
	use super::*;
	pub trait Sealed<M> {}
	impl<T> Sealed<marker::One> for T {}
	#[impl_tuple(2..=16)]
	impl Sealed<marker::Many> for Each!(T) {}
}

impl<T> Tuple<marker::One> for T {
	type Ref<'l> = &'l T where Self: 'l;
	type Mut<'l> = &'l mut T where Self: 'l;
	fn take_ref(&self) -> Self::Ref<'_> {self}
}

#[impl_tuple(2..=16)]
impl Tuple<marker::Many> for Each!(T) {
	type Ref<'l> = Each!(T => &'l T) where Self: 'l;
	type Mut<'l> = Each!(T => &'l mut T) where Self: 'l;

	/// `&(A, B, C)` → `(&A, &B, &C)`
	fn take_ref(&self) -> Self::Ref<'_> {
		each!{ref field => field}
	}
}

