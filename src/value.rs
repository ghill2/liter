//! Datatypes that consist of one or more [`Column`]s and make up [`Table`]s

use construe::StrConstrue;

use crate::{
	Column,
	Bind,
	Fetch
};
use crate::column::ColumnDef;
use crate::table::{
	Table,
	HasKey
};

/// A (compound) datatype that can be used in a [`Table`]
pub trait Value: Bind + Fetch {
	const DEFINITION: ValueDef;
	const COLUMN_COUNT: usize = Self::DEFINITION.inner.count_columns();
	type References;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValueDef {
	/// `UNIQUE` constraint across *all* constituent [`Column`]s
	pub unique: bool,
	/// Whether constituent [`Column`]s should be marked `NOT NULL`
	///
	/// Note that this constraint doesn't strictly match Rust semantics, because it applies to *each* column individually and not all of them together.
	/// So, in terms of the constraint, for a [`Value`] `V` with three [`Column`]s `A`, `B`, and `C`, `Option<V>` is not analogous to `Option<(A, B, C)>`, but rather `(Option<A>, Option<B>, Option<C>)`.
	pub nullable: bool,
	pub inner: NestedValueDef,
	pub reference: Option<ForeignKey>,
	pub checks: &'static [Check]
}

#[derive(Debug, PartialEq, Eq)]
pub enum NestedValueDef {
	Column (ColumnDef),
	//Columns (&'static [(&'static str, ColumnDef)]),
	Value (&'static ValueDef),
	Values (&'static [(&'static str, ValueDef)]),
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
	// SQL string that will be prepended with the name of the column
	Sql(&'static str)
}

// Note: The Value does not know the Type that is being referenced
#[derive(Debug, PartialEq, Eq)]
pub struct ForeignKey {
	pub table_name: &'static str,
	pub deferrable: bool,
	pub on_delete: FkConflictAction,
	pub on_update: FkConflictAction
}

impl ForeignKey {
	pub const fn define_for<T: Table + HasKey>() -> Self {
		Self {
			table_name: T::NAME,
			deferrable: true,
			on_delete: FkConflictAction::Restrict,
			on_update: FkConflictAction::Restrict
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FkConflictAction {
	Cascade,
	Restrict,
	SetNull
}

/// Linked list of [`&str`]
///
/// Instead of requiring a variably sized collection to store the set of names for each column, store them on the stack as a linked list.
/// While the size of each type on the stack must be known at compile-time, the depth of recursion does not.
/// This allows storing a variable number of items on the stack: each recursive invocation receives a reference to the currently stored link and stores it as well, forming a linked list.
pub(crate) struct StrChain<'l> {
	name: &'l str,
	link: Option<&'l StrChain<'l>>
}

impl<'l> StrChain<'l> {
	pub const fn start(name: &'l str) -> Self {
		Self { name, link: None }
	}
	pub const fn with(&'l self, name: &'l str) -> Self {
		Self { name, link: Some(self) }
	}

	pub const fn join<const L: usize>(
		&self,
		mut construe: StrConstrue<L>,
		separator: &str)
		-> StrConstrue<L>
	{
		if let Some(prev) = self.link.as_ref() {
			construe = prev.join(construe, separator);
			construe = construe.push_str(separator);
		}
		construe.push_str(self.name)
	}
}

/*
 *	DEFINITION ASSEMBLY
 */

impl ForeignKey {
	pub(crate) const fn push_sql<const N: usize>(&self, mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		sc = sc.push_str(" REFERENCES ");
		sc = sc.push_str(self.table_name);
		sc = sc.push_str("\n\t\tON UPDATE ");
		sc = sc.push_str(self.on_update.as_sql());
		sc = sc.push_str("\n\t\tON DELETE ");
		sc = sc.push_str(self.on_delete.as_sql());
		if self.deferrable {
			sc = sc.push_str("\n\t\tDEFERRABLE INITIALLY DEFERRED");
		}
		sc
	}
}


impl FkConflictAction {
	const fn as_sql(self) -> &'static str {
		match self {
			Self::Cascade => "CASCADE",
			Self::Restrict => "RESTRICT",
			Self::SetNull => "SET NULL",
		}
	}
}

impl<T: Column> Value for T {
	type References = ();
	//type Columns = Self;
	const DEFINITION: ValueDef = ValueDef {
		unique: false,
		nullable: false,
		inner: NestedValueDef::Column(<Self as Column>::DEFINITION),
		reference: None,
		checks: &[],
	};
}

impl<T: Value> Value for Option<T> {
	type References = T::References;
	const DEFINITION: ValueDef = ValueDef {
		nullable: true,
		..T::DEFINITION
	};
}

impl ValueDef {
	/// Override the `unique` field as `true`
	pub const fn unique(self) -> Self {
		Self {unique: true, ..self}
	}
	pub(crate) const fn push_sql<const N: usize>(
		&self,
		name: &str,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		sc = self.inner.push_sql(self.nullable, &StrChain::start(name), sc);
		if self.unique && self.inner.count_columns() == 1 {
			sc = sc.push_str(" UNIQUE");
		}
		sc

	}
	pub(crate) const fn push_constraint_sql<const N: usize>(
		&self,
		chain: &StrChain<'_>,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		// if inner is single-column: append UNIQUE to the column instead
		if self.unique && self.inner.count_columns() != 1 {
			sc = sc.push_str(",\n\tUNIQUE (");
			sc = self.inner.push_column_names(chain, sc);
			sc = sc.push_str(")");
		}
		if let Some(ref fk_ref) = self.reference {
			sc = sc.push_str(",\n\tFOREIGN KEY (");
			sc = self.inner.push_column_names(chain, sc);
			sc = sc.push_str(")");
			sc = fk_ref.push_sql(sc)
		}
		match self.inner {
			NestedValueDef::Column(_) => {},
			NestedValueDef::Value(v) => sc = v.push_constraint_sql(chain, sc),
			NestedValueDef::Values(mut values) => {
				while let [(name, def), rest @ ..] = values {
					values = rest;
					sc = def.push_constraint_sql(&chain.with(name), sc);
				}
			}
		}
		/*
		TODO: CHECK CONSTRAINTS
		*/
		sc
	}
}

impl NestedValueDef {
	pub(crate) const fn push_column_names<const N: usize>(
		&self,
		chain: &StrChain<'_>,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		match self {
			Self::Column(_def) => chain.join(sc, "_"),
			Self::Value(def) => def.inner.push_column_names(chain, sc),
			// this matches only on the last definition
			Self::Values([(name, def)]) =>
				def.inner.push_column_names(&chain.with(name), sc),
			// this would also match on the last definition, so it comes after
			Self::Values([first, rest @ ..]) => {
				let (name, def) = first;
				// this descends
				sc = def.inner.push_column_names(&chain.with(name), sc);
				sc = sc.push_str(", ");
				// this doesn't actually descend (yet), it's just unpacking
				Self::Values(rest).push_column_names(chain, sc)
			},
			Self::Values([]) => panic!("empty Values([])")
		}
	}
	pub(crate) const fn push_sql<const N: usize>(
		&self,
		nullable: bool,
		chain: &StrChain<'_>,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		match self {
			Self::Column(def) if nullable => def.nullable().push_sql(chain, sc),
			Self::Column(def) => def.push_sql(chain, sc),
			Self::Value(def) => def.inner
				.push_sql(nullable | def.nullable, chain, sc),
			// this matches only on the last definition
			Self::Values([(name, def)]) => def.inner.
				push_sql(nullable | def.nullable, &chain.with(name), sc),
			// this would also match on the last definition, so it comes after
			Self::Values([(first_name, first_def), rest @ ..]) => {
				// this descends
				sc = first_def.inner.push_sql(
					nullable | first_def.nullable,
					&chain.with(first_name),
					sc
				);
				sc = sc.push_str(",\n\t");
				// this doesn't actually descend (yet), it's just unpacking
				Self::Values(rest).push_sql(nullable, chain, sc)
			},
			Self::Values([]) => panic!("empty Values([])")
		}
	}
	const fn count_columns(&self) -> usize {
		match self {
			// base case
			Self::Column(_def) => 1,
			// multi-column Value implementation on a struct
			// define each with name prepended
			//Self::Columns(_) => todo!(),
			// Single-key Ref
			// recurse with name
			Self::Value(def) => def.inner.count_columns(),
			// Composite-Key Ref
			// recurse with name + subname
			Self::Values([(_, first), rest @ ..]) =>
				first.inner.count_columns() + Self::Values(rest).count_columns(),
			Self::Values([]) => 0
		}
	}
}


