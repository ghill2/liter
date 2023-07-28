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

pub trait Value: Bind + Fetch {
	const DEFINITION: ValueDef;
	const COLUMN_COUNT: usize = Self::DEFINITION.inner.count_columns();
	type References;
}

#[derive(Debug)]
pub struct ValueDef {
	pub unique: bool,
	//pub nullable: bool,
	pub inner: InnerValueDef,
	pub reference: Option<ForeignKey>,
	pub checks: &'static [Check]
}

#[derive(Debug)]
pub enum InnerValueDef {
	Column (ColumnDef),
	//Columns (&'static [(&'static str, ColumnDef)]),
	Value (&'static InnerValueDef),
	Values (&'static [(&'static str, InnerValueDef)]),
}


#[derive(Clone, Copy, Debug)]
pub enum Check {
	// SQL string that will be prepended with the name of the column
	Sql(&'static str)
}

// Note: The Value does not know the Type that is being referenced
#[derive(Debug)]
pub struct ForeignKey {
	pub table_name: &'static str,
	pub deferrable: bool,
	pub on_delete: FkConflictAction,
	pub on_update: FkConflictAction
}

impl ForeignKey {
	pub fn define_for<T: Table + HasKey>() -> Self {
		Self {
			table_name: T::NAME,
			deferrable: false,
			on_delete: FkConflictAction::Restrict,
			on_update: FkConflictAction::Restrict
		}
	}
}

#[derive(Clone, Copy, Debug)]
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
	pub(crate) fn write_sql_to(&self, sql: &mut String) {
		sql.push_str(" REFERENCES ");
		sql.push_str(self.table_name);
		sql.push_str(" ON UPDATE ");
		sql.push_str(self.on_update.as_sql());
		sql.push_str(" ON DELETE ");
		sql.push_str(self.on_delete.as_sql());
		if self.deferrable {
			sql.push_str(" DEFERRABLE INITIALLY DEFERRED");
		}
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
		inner: InnerValueDef::Column(<Self as Column>::DEFINITION),
		reference: None,
		checks: &[],
	};
}

impl ValueDef {
	pub fn define(
		&self,
		name: &str,
		into: &mut String,
		constraints: &mut String)
	{
		// define columns
		self.inner.define(name, into);

		if self.unique {
			// TODO: if inner is single-column: append UNIQUE instead
			constraints.push_str(",\n\tUNIQUE (");
			self.inner.write_column_names(name, constraints);
			constraints.push(')');
		}
		if let Some(ref fk_ref) = self.reference {
			constraints.push_str(",\n\tFOREIGN KEY (");
			self.inner.write_column_names(name, constraints);
			constraints.push(')');
			fk_ref.write_sql_to(constraints)
		}
		for Check::Sql(check) in self.checks {
			// How to do checks for multi-column values?!
			// (For now): Just SQL, no name help
			// could do "template strings" or special structs
			constraints.push_str(",\n\tCHECK (");
			constraints.push_str(check);
			constraints.push(')');
		}
	}
	pub const fn push_constraint_sql<const N: usize>(
		&self,
		name: &str,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		if self.unique {
			// TODO: if inner is single-column: append UNIQUE instead
			sc = sc.push_str(",\n\tUNIQUE (");
			sc = self.inner.push_column_names(&StrChain::start(name), sc);
			sc = sc.push_str(")");
		}
		if let Some(ref fk_ref) = self.reference {
			sc = sc.push_str(",\n\tFOREIGN KEY (");
			sc = self.inner.push_column_names(&StrChain::start(name), sc);
			sc = sc.push_str(")");
			sc = fk_ref.push_sql(sc)
		}
		/*
		TODO: CHECK CONSTRAINTS
		*/
		sc
	}
}

impl InnerValueDef {
	pub(crate) const fn push_column_names<const N: usize>(
		&self,
		chain: &StrChain<'_>,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		match self {
			Self::Column(_def) => chain.join(sc, "_"),
			Self::Value(def) => def.push_column_names(chain, sc),
			// this matches only on the last definition
			Self::Values([(name, def)]) =>
				def.push_column_names(&chain.with(name), sc),
			// this would also match on the last definition, so it comes after
			Self::Values([(first_name, first_def), rest @ ..]) => {
				// this descends
				sc = first_def.push_column_names(&chain.with(first_name), sc);
				sc = sc.push_str(", ");
				// this doesn't actually descend (yet), it's just unpacking
				Self::Values(rest).push_column_names(chain, sc)
			},
			Self::Values([]) => panic!("empty Values([])")
		}
	}
	pub(crate) const fn push_sql<const N: usize>(
		&self,
		chain: &StrChain<'_>,
		mut sc: StrConstrue<N>)
		-> StrConstrue<N>
	{
		match self {
			Self::Column(def) => def.push_sql(chain, sc),
			Self::Value(def) => def.push_sql(chain, sc),
			// this matches only on the last definition
			Self::Values([(name, def)]) => def.push_sql(&chain.with(name), sc),
			// this would also match on the last definition, so it comes after
			Self::Values([(first_name, first_def), rest @ ..]) => {
				// this descends
				sc = first_def.push_sql(&chain.with(first_name), sc);
				sc = sc.push_str(",\n\t");
				// this doesn't actually descend (yet), it's just unpacking
				Self::Values(rest).push_sql(chain, sc)
			},
			Self::Values([]) => panic!("empty Values([])")
		}
	}
	pub fn write_column_names(&self, name: &str, into: &mut String) {
		match self {
			// base case
			InnerValueDef::Column(_def) => into.push_str(name),
			// multi-column Value implementation on a struct
			// define each with name prepended
			//InnerValueDef::Columns(_) => todo!(),
			// Single-key Ref
			// recurse with name
			InnerValueDef::Value(def) => def.write_column_names(name, into),
			// Composite-Key Ref
			// recurse with name + subname
			InnerValueDef::Values([(first_name, first_def), rest @ ..]) => {
				first_def.write_column_names(
					&format!("{name}_{first_name}"),
					into
				);
				for (sub_name, def) in rest.iter() {
					into.push_str(", ");
					def.write_column_names(&format!("{name}_{sub_name}"), into)
				}
			},
			// if this were allowed to be empty, above code for adding ", " would have to be changed
			InnerValueDef::Values([]) => unreachable!()
		}
	}
	pub fn define(&self, name: &str, into: &mut String) {
		match self {
			// base case
			InnerValueDef::Column(def) => def.write_sql_to(name, into),
			// multi-column Value implementation on a struct
			// define each with name prepended
			//InnerValueDef::Columns(_) => todo!(),
			// Single-key Ref
			// recurse with name
			InnerValueDef::Value(def) => def.define(name, into),
			// Composite-Key Ref
			// recurse with name + subname
			InnerValueDef::Values([(first_name, first_def), rest @ ..]) => {
				first_def.define(
					&format!("{name}_{first_name}"),
					into
				);
				for (sub_name, def) in rest.iter() {
					into.push_str(",\n\t");
					def.define(&format!("{name}_{sub_name}"), into)
				}
			},
			// if this were allowed to be empty, above code for adding ",\n\t" would have to be changed
			InnerValueDef::Values([]) => unreachable!()
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
			Self::Value(def) => def.count_columns(),
			// Composite-Key Ref
			// recurse with name + subname
			Self::Values([(_, first), rest @ ..]) =>
				first.count_columns() + Self::Values(rest).count_columns(),
			Self::Values([]) => 0
		}
	}
}


