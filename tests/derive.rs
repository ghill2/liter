use liter::{
	Table,
	Ref,
	database,
	Value,
	Entry,
	Bind,
	Fetch
};
use liter::value::NestedValueDef;
use rusqlite::Result as SqlResult;

macro_rules! contains {
	($t:ty, $( $c:literal ),*) => {
		$(
			assert!(<$t>::CREATE_TABLE.contains($c), "{}", <$t>::CREATE_TABLE);
		)*
	};
}
macro_rules! fetch {
	($db:expr, $c:literal) => {
		$db.query_row($c, [], Fetch::from_row)
	};
}

#[test]
fn from_struct() -> SqlResult<()> {

	//TODO:
	// checks
	// unique constraints

	#[derive(Value)]
	struct Point {
		timestamp: i64
	}
	assert_eq!(
		Point::DEFINITION.inner,
		NestedValueDef::Value(&u64::DEFINITION)
	);

	#[derive(Table)]
	struct PointTable {
		point: Point
	}
	contains!(PointTable, "point INTEGER NOT NULL");

	#[derive(Value)]
	struct Frame {
		start: Point,
		stop: Point
	}
	assert_eq!(
		Frame::DEFINITION.inner,
		NestedValueDef::Values(&[
			("start", Point::DEFINITION),
			("stop", Point::DEFINITION)
		])
	);
	assert_eq!(Frame::COLUMNS, 2);

	#[derive(Table)]
	struct FrameTable {
		#[key]
		frame: Frame
	}
	contains!(
		FrameTable,
		"frame_start INTEGER NOT NULL",
		"frame_stop INTEGER NOT NULL",
		"PRIMARY KEY ( frame_start, frame_stop )"
	);

	#[derive(Value)]
	struct Update {
		on: Ref<FrameTable>,
		new: Frame
	}

	assert_eq!(Update::COLUMNS, 4);

	#[derive(Table)]
	struct UpdateTable {
		update: Update
	}
	contains!(
		UpdateTable,
		"FOREIGN KEY (update_on_start, update_on_stop) REFERENCES frametable"
	);

	#[database]
	struct Database(PointTable, FrameTable, UpdateTable);

	let db = Database::create_in_memory()?;

	assert!(db.get_all::<PointTable>()?.is_empty());
	assert!(db.get_all::<FrameTable>()?.is_empty());
	assert!(db.get_all::<UpdateTable>()?.is_empty());

	let point = Point {timestamp: 123};
	assert_eq!(db.insert(&PointTable{point})?, 1);
	let frame = Frame {
		start: Point {timestamp: 12345},
		stop: Point {timestamp: 23456}
	};
	assert_eq!(db.insert(&FrameTable{frame})?, 1);


	Ok(())
}

#[test]
fn from_enum() -> SqlResult<()> {

	#[derive(Value, PartialEq, Eq, Debug)]
	enum Abc {
		A,
		B,
		C
	}
	assert_eq!(
		Abc::DEFINITION.inner,
		NestedValueDef::Value(&String::DEFINITION)
	);

	#[derive(Value, PartialEq, Debug)]
	enum X {
		String(String),
		Struct {
			u8: u8,
			array: [u8; 3],
			//tuple: (usize, usize)
		},
		Tuple (f64, f64),
		Enum(Abc),
		Unit
	}

	#[derive(Table)]
	struct AbcX {
		abc: Abc,
		x: X
	}

	#[database]
	struct Db(AbcX);

	let db = Db::create_in_memory()?;

	assert_eq!(Abc::A, db.query_row("SELECT 'A'", [], Fetch::from_row)?);
	assert_eq!(Abc::B, db.query_row("SELECT 'B'", [], Fetch::from_row)?);
	assert_eq!(Abc::C, db.query_row("SELECT 'C'", [], Fetch::from_row)?);
	assert_eq!(Some(Abc::A), db.query_row("SELECT 'A'", [], Fetch::from_row)?);
	assert_eq!(Some(Abc::B), db.query_row("SELECT 'B'", [], Fetch::from_row)?);
	assert_eq!(Some(Abc::C), db.query_row("SELECT 'C'", [], Fetch::from_row)?);
	assert_eq!(None::<Abc>, db.query_row("SELECT NULL", [], Fetch::from_row)?);
	assert!(db.query_row("SELECT 'D'", [], Abc::from_row).is_err());

	assert_eq!(
		X::String("abc123".to_string()),
		fetch!(db, "SELECT 'String', 'abc123', NULL, NULL, NULL, NULL, NULL")?
	);
	assert_eq!(
		X::Struct { u8: 123, array: [0; 3] },
		fetch!(db, "SELECT 'Struct', NULL, 123, x'000000', NULL, NULL, NULL")?
	);
	assert_eq!(
		X::Tuple(3.3, 99.0),
		fetch!(db, "SELECT 'Tuple', NULL, NULL, NULL, 3.3, 99, NULL")?
	);
	assert_eq!(
		X::Enum(Abc::C),
		fetch!(db, "SELECT 'Enum', NULL, NULL, NULL, NULL, NULL, 'C'")?
	);
	assert_eq!(
		X::Unit,
		fetch!(db, "SELECT 'Unit', NULL, NULL, NULL, NULL, NULL, NULL")?
	);
	assert_eq!(
		None::<X>,
		fetch!(db, "SELECT NULL, NULL, NULL, NULL, NULL, NULL, NULL")?
	);
	assert!(matches!(
		fetch!(db, "SELECT 'Unit', NULL, NULL, x'000000', NULL, NULL, NULL"),
		Err::<X, _>(rusqlite::Error::FromSqlConversionFailure(4, _, _))
	));
	assert!(matches!(
		fetch!(db, "SELECT 'Unit', NULL, NULL, NULL, 0, NULL, NULL"),
		Err::<X, _>(rusqlite::Error::FromSqlConversionFailure(5, _, _))
	));
	assert!(matches!(
		fetch!(db, "SELECT 'Unit', NULL, NULL, NULL, NULL, 0, NULL"),
		Err::<X, _>(rusqlite::Error::FromSqlConversionFailure(6, _, _))
	));


	let tuple_x = X::Tuple(f64::INFINITY, 123.456);
	assert_eq!(
		tuple_x,
		db.query_one_with("SELECT ?, ?, ?, ?, ?, ?, ?", &tuple_x)?
	);
	assert_eq!(
		None::<X>,
		db.query_one_with("SELECT ?, ?, ?, ?, ?, ?, ?", &None::<X>)?
	);

	assert_eq!(db.execute(AbcX::INSERT, &(Abc::C, tuple_x))?, 1);

	Ok(())
}

