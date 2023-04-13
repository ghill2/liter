use crate::{
	Schema,
	Table
};

pub trait PartOf<S: Schema> {}
impl<S: Schema> PartOf<S> for () {}

pub trait AllPartOf<S: Schema> {}
//impl<S: Schema> AllPartOf<S> for () {}
impl<S: Schema, T: PartOf<S>> AllPartOf<S> for (T, ) {}
impl<S: Schema, T: PartOf<S>, L: AllPartOf<S>> AllPartOf<S> for (T, L) {}


/// Checks whether a table is valid to use in a given schema.
/// A Table is valid IFF all of it's fields only reference tables that are also part of the Schema.
///
/// "Unrolls" the nested type.
pub trait ValidFor<S: Schema> {}

impl<S: Schema> ValidFor<S> for () {}
impl<S: Schema, T: Table> ValidFor<S> for T
	where T: PartOf<S>
{}
impl<S: Schema, T: Table, L> ValidFor<S> for (T, L)
	where T: ValidFor<S>,
		L: ValidFor<S>
{}


pub trait RefsArePartOf<S: Schema> {}
impl<S: Schema, T: Table> RefsArePartOf<S> for T
	where T::References: ValidFor<S>
{}

pub trait IsValidFor<S: Schema>: RefsArePartOf<S> {}

