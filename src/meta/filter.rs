use crate::table::Entry;

pub trait Filter {
	type F;
}

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
pub type Filtered<T> = <T as Filter>::F;

