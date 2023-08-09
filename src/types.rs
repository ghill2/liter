mod bind;
mod fetch;

pub use bind::{
	Bind,
	Binder,
	ToSql2
};

pub use fetch::{
	Fetch,
	Fetcher,
	FromSql2
};

#[doc(hidden)]
#[macro_export]
macro_rules! impl_from_to_sql_2 {
	($t:ty) => {
		impl $crate::types::FromSql2 for $t {}
		impl $crate::types::ToSql2 for $t {}
	};
}
#[doc(inline)]
pub use impl_from_to_sql_2;
