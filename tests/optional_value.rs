use liter::{
	Id,
	Table,
	database,
	Fetch
};
use rusqlite::Result as SqlResult;


#[test]
fn fetch() -> SqlResult<()> {
	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Item {
		#[key]
		id: Id,
		data: u64
	}
	let db = Db::create_in_memory()?;

	let (a, b, c): (u8, String, f64) = db.query_row(
		"SELECT 1, '2', 3.0",
		[],
		Fetch::from_row
	)?;

	assert_eq!(a, 1);
	assert_eq!(b, "2");
	assert_eq!(c, 3.0);

	let opt_abc: Option<(u8, String, f64)> = db.query_row(
		"SELECT 1, '2', 3.0",
		[],
		Fetch::from_row
	)?;

	assert_eq!(opt_abc, Some((a, b, c)));

	Ok(())
}
