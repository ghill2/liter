use rusqlite::types::{
	ToSql,
	ToSqlOutput,
	FromSql,
	FromSqlResult,
	ValueRef
};
use liter::{
	Table,
	Entry,
	Column,
	bind::ToSql2,
	column::Affinity,
	fetch::FromSql2,
	value::Check,
	database,
};

#[test]
fn primitives() {
	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq)]
	struct Item {
		int: u64,
		float: f64,
		text: String,
		blob: [u8; 16]
	}

	let defs = [
		"int INTEGER NOT NULL",
		"float REAL NOT NULL",
		"text TEXT NOT NULL",
		"blob BLOB NOT NULL",
	];

	for snippet in defs {
		assert!(
			Item::CREATE_TABLE.contains(snippet),
			"Table definition did not contain {snippet:?}",
		);
	}
	let values = (
		3030,
		f64::INFINITY,
		":^)",
		999_999u128.to_le_bytes()
	);
	let v = &values;

	let db = Db::create_in_memory().unwrap();

	macro_rules! assert_err {
		($reason:expr, $sql:expr, $( $v:expr ),*) => {
			$( db.execute($sql, $v).expect_err($reason); )*
		};
	}

	assert_err!(
		"insert None into non-null column",
		Item::INSERT,
		&(None::<u8>, v.1, v.2, v.3),
		&(v.0, None::<u8>, v.2, v.3),
		&(v.0, v.1, None::<u8>, v.3),
		&(v.0, v.1, v.2, None::<u8>)
	);

	db.execute(Item::INSERT, v).unwrap();

	println!("Checking strict affinity enforcement");
	assert!(Item::CREATE_TABLE.contains("STRICT"));

	// Some of these don't error because SQLite tries to coerce the data
	// https://www.sqlite.org/stricttables.html:
	// "If the value cannot be losslessly converted in the specified datatype, then an SQLITE_CONSTRAINT_DATATYPE error is raised."

	// this is not a great test, behaviour may change

	//assert_err!("int => text", Item::INSERT, &(v.0, v.1, v.0, v.3));
	assert_err!("int => blob", Item::INSERT, &(v.0, v.1, v.2, v.0));
	//assert_err!("int => real", Item::INSERT, &(v.0, v.0, v.2, v.3));
	// this one only fails because the real happens to be INFINITY
	assert_err!("real => int", Item::INSERT, &(v.1, v.1, v.2, v.3));
	//assert_err!("real => text", Item::INSERT, &(v.0, v.1, v.1, v.3));
	assert_err!("real => blob", Item::INSERT, &(v.0, v.1, v.2, v.1));
	assert_err!("text => int", Item::INSERT, &(v.2, v.1, v.2, v.3));
	assert_err!("text => real", Item::INSERT, &(v.0, v.2, v.2, v.3));
	assert_err!("text => blob", Item::INSERT, &(v.0, v.1, v.2, v.2));
	assert_err!("blob => int", Item::INSERT, &(v.3, v.1, v.2, v.3));
	assert_err!("blob => real", Item::INSERT, &(v.0, v.3, v.2, v.3));
	assert_err!("blob => text", Item::INSERT, &(v.0, v.1, v.3, v.3));
}

#[test]
fn nullable() {
	#[database]
	struct Db (Item);

	#[derive(Default, Table, Clone, Debug, PartialEq)]
	struct Item {
		int: Option<u64>,
		float: Option<f64>,
		text: Option<String>,
		blob: Option<[u8; 16]>
	}

	let defs = [
		"int INTEGER",
		"float REAL",
		"text TEXT",
		"blob BLOB",
	];

	for snippet in defs {
		assert!(
			Item::CREATE_TABLE.contains(snippet),
			"Table definition did not contain {snippet:?}",
		);
	}
	assert!(!Item::CREATE_TABLE.contains("NOT NULL"));

	let db = Db::create_in_memory().unwrap();
	db.insert(&Item::default()).unwrap();
}

#[test]
fn check_even_number() {
	#[derive(Default, Clone, Debug, PartialEq)]
	struct EvenNumber(u64);
	impl Column for EvenNumber {
		const AFFINITY: Affinity = Affinity::Integer;
		const NULLABLE: bool = false;
		const CHECKS: &'static [Check] = &[
			Check::Sql("% 2 = 0"),
			Check::Sql(">= 0")
		];
	}
	impl ToSql for EvenNumber {
		fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
			self.0.to_sql()
		}
	}
	impl FromSql for EvenNumber {
		fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
			u64::column_result(value).map(Self)
		}
	}
	impl ToSql2 for EvenNumber {}
	impl FromSql2 for EvenNumber {}


	#[database]
	struct Db (Item);

	#[derive(Default, Table, Clone, Debug, PartialEq)]
	struct Item {
		even: EvenNumber
	}
	println!("{}", Item::CREATE_TABLE);
	assert!(Item::CREATE_TABLE.contains("even % 2 = 0"));

	let db = Db::create_in_memory().unwrap();

	for i in 0..16 {
		let item = Item {even: EvenNumber(i)};
		println!("{i}");
		if i % 2 == 0 {
			db.insert(&item).unwrap();
		}
		else {
			db.insert(&item).unwrap_err();
		}
	}
	db.execute("INSERT INTO item VALUES (?)", &-2).unwrap_err();
}

#[test]
fn check_short_string() {
	#[derive(Default, Clone, Debug, PartialEq)]
	struct ShortString(String);
	impl Column for ShortString {
		const AFFINITY: Affinity = Affinity::Text;
		const NULLABLE: bool = false;
		const CHECKS: &'static [Check] = &[
			// TODO: use length(<column_name>) < 10 when possible
			Check::Sql("NOT LIKE \"__________%\"")
		];
	}
	impl ToSql for ShortString {
		fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
			self.0.to_sql()
		}
	}
	impl FromSql for ShortString {
		fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
			String::column_result(value).map(Self)
		}
	}
	impl ToSql2 for ShortString {}
	impl FromSql2 for ShortString {}


	#[database]
	struct Db (Item);

	#[derive(Default, Table, Clone, Debug, PartialEq)]
	struct Item {
		short: ShortString
	}
	assert!(Item::CREATE_TABLE.contains("short NOT LIKE"));

	let db = Db::create_in_memory().unwrap();

	let fit = [
		"short",
		"ShortStr",
		"",
		"a",
		"123456789"
	];
	for s in fit {
		let item = Item {short: ShortString(s.to_string())};
		db.insert(&item).expect(s);
	}
	let dont_fit = [
		"not quite short enough",
		"ShortString",
		"1234567890",
		"12345678901",
		"VeryLongString"
	];
	for s in dont_fit {
		let item = Item {short: ShortString(s.to_string())};
		db.insert(&item).expect_err(s);
	}
}
