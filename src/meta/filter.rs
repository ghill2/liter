use crate::table::Entry;

/// Strip `()`s from nested [`Entry`] tuple types
pub trait Filter {
	type F;
}
/// Type-alias that strips `()`s from nested [`Entry`] tuple types
///
/// This is just a more convenient way to use the [`Filter`] trait.
pub type Filtered<T> = <T as Filter>::F;

impl Filter for ((), ) {
	type F = ();
}
impl<E: Entry> Filter for E {
	type F = E;
}
impl<E: Entry> Filter for (E, ) {
	type F = E;
}
impl<E: Entry> Filter for (E, ()) {
	type F = E;
}
impl<T: Filter> Filter for ((), T) {
	type F = T::F;
}
impl<E: Entry, T: Filter> Filter for (E, T) {
	type F = (E, T::F);
}
