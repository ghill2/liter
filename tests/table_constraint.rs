use liter::{
	Entry,
	Id,
	Table,
	Value,
	database,
};


#[test]
fn single_primary_key() -> rusqlite::Result<()> {
	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Item {
		#[key]
		id: Id,
		data: u64
	}
	assert!(
		Item::CREATE_TABLE.contains("PRIMARY KEY"),
		"definition must include PRIMARY KEY clause"
	);

	let db = Db::create_in_memory()?;
	assert!(db.get_all::<Item>()?.is_empty());
	let mut item = Item {id: Id::NULL, data: 123};
	db.create(&mut item)?;
	assert_ne!(item.id, Id::NULL, "inserting must change id");

	let mut item_2 = item.clone();
	db.insert(&item_2).expect_err(
		"inserting with same Id should violate primary key constraint and fail"
	);

	item_2.data = 99999999;
	assert_eq!(db.update(&item_2)?, 1);

	let updated_item: Item = db.get(item_2.id.clone())?.unwrap();
	assert_eq!(updated_item, item_2);

	db.insert(&item_2).expect_err(
		"inserting with same Id should violate primary key constraint and fail"
	);

	// upsert replace
	item_2.data = 33333333;
	assert_eq!(db.upsert(&item_2)?, 1);

	let upserted_item: Item = db.get(item_2.id.clone())?.unwrap();
	assert_eq!(upserted_item, item_2);

	assert!(db.delete::<Item>(&item_2.id)?);
	assert!(db.get::<Item>(item_2.id)?.is_none());
	assert!(db.get_all::<Item>()?.is_empty());

	// upsert uncontested ID
	let item_3 = Item {
		id: Id::from_i64(12345),
		data: 123456789
	};
	assert_eq!(db.upsert(&item_3)?, 1);

	let upserted_item: Item = db.get(item_3.id.clone())?.unwrap();
	assert_eq!(upserted_item, item_3);

	// upsert new (NULL) ID
	let item_4 = Item {
		id: Id::NULL,
		data: 5
	};
	assert_eq!(db.upsert(&item_4)?, 1);

	let item_upserted = db.get_all::<Item>()?
		.drain(..)
		.any(|item| item.data == item_4.data);
	assert!(item_upserted);

	assert_eq!(db.get_all::<Item>()?.len(), 2);

	Ok(())
}


#[test]
fn composite_primary_key() -> rusqlite::Result<()> {
	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Item {
		#[key]
		id: u64,
		#[key]
		parent_id: String,
		data: u64
	}
	assert!(
		Item::CREATE_TABLE.contains("PRIMARY KEY"),
		"definition must include PRIMARY KEY clause"
	);

	let db = Db::create_in_memory()?;
	assert!(db.get_all::<Item>()?.is_empty());
	let item = Item {
		id: 10,
		parent_id: "12".to_string(),
		data: 123
	};
	assert_eq!(db.insert(&item)?, 1, "failed to insert");

	assert_eq!(db.get_all::<Item>()?.pop().unwrap(), item);

	let item_2 = item.clone();
	db.insert(&item_2).expect_err(
		"inserting with same key should violate primary key constraint and fail"
	);

	let mut item = db.get_all::<Item>()?.pop().unwrap();
	item.data = 999999999;
	assert_eq!(db.update(&item)?, 1);

	let updated_item = db.get_all::<Item>()?.pop().unwrap();
	assert_eq!(item, updated_item);

	db.insert(&item_2).expect_err(
		"inserting with same key should violate primary key constraint and fail"
	);

	item.data = 3333333333;
	assert_eq!(db.upsert(&item)?, 1);

	let upserted_item = db.get_all::<Item>()?.pop().unwrap();
	assert_eq!(item, upserted_item);

	use liter::HasKey;
	let key = item.make_ref().0;

	assert!(db.delete::<Item>(&key)?);
	assert!(db.get::<Item>(key)?.is_none());
	assert!(db.get_all::<Item>()?.is_empty());

	let item_2 = Item {
		id: 123,
		parent_id: "abc".to_string(),
		data: 42
	};
	assert_eq!(db.upsert(&item_2)?, 1, "failed to upsert");
	assert_eq!(db.get_all::<Item>()?.pop().unwrap(), item_2);

	Ok(())
}


#[test]
fn check() -> rusqlite::Result<()> {
	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	#[check("data <= 9999")]
	#[check("id BETWEEN 5 AND 15")]
	struct Item {
		id: u8,
		data: u64
	}
	println!("{}", Item::CREATE_TABLE);
	assert!(
		Item::CREATE_TABLE.contains("CHECK (data <= 9999)"),
		"definition must include CHECK clause"
	);
	assert!(
		Item::CREATE_TABLE.contains("CHECK (id BETWEEN 5 AND 15)"),
		"definition must include CHECK clause"
	);

	let db = Db::create_in_memory()?;
	assert!(db.get_all::<Item>()?.is_empty());
	let item = Item {
		id: 10,
		data: 123
	};
	assert_eq!(db.insert(&item)?, 1, "failed to insert");

	assert_eq!(db.get_all::<Item>()?.pop().unwrap(), item);

	let item_2 = Item {
		id: 12,
		data: 10_000
	};
	db.insert(&item_2).expect_err("first CHECK constraint should be violated");

	let item_3 = Item {
		id: 16,
		data: 9999
	};
	db.insert(&item_3).expect_err("second CHECK constraint should be violated");

	let item_3 = Item {
		id: 20,
		data: 3_000_000
	};
	db.insert(&item_3).expect_err("both CHECK constraints should be violated");

	Ok(())
}

#[test]
fn unique() -> rusqlite::Result<()> {
	#[database]
	struct Db (OnTable, OnField, MultiTable);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	#[unique(number)]
	struct OnTable {
		number: u8
	}
	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct OnField {
		#[unique]
		number: Option<u8>
	}

	assert!(
		OnTable::CREATE_TABLE.contains("UNIQUE"),
		"definition must include UNIQUE constraint"
	);
	assert!(
		OnField::CREATE_TABLE.contains("number INTEGER UNIQUE"),
		"definition must include inline UNIQUE constraint"
	);

	// check that SQL is valid
	let db = Db::create_in_memory()?;

	assert!(db.get_all::<OnTable>()?.is_empty());
	let item = OnTable {number: 9};
	assert_eq!(db.insert(&item).unwrap(), 1);
	let opt_item: Option<OnTable> = db.query_one(OnTable::GET_ALL)?;
	assert_eq!(opt_item.as_ref(), Some(&item));

	db.insert(&item).expect_err(
		"inserting the same number should violate unique constraint and fail"
	);

	assert!(db.get_all::<OnField>()?.is_empty());
	let opt_item = OnField {number: Some(9)};
	assert_eq!(db.insert(&opt_item).unwrap(), 1);

	let opt_item_2 = opt_item.clone();
	db.insert(&opt_item_2).expect_err(
		"inserting the same number should violate unique constraint and fail"
	);

	let none_item = OnField {number: None};
	assert_eq!(db.insert(&none_item).unwrap(), 1);


	#[derive(Value, Clone, Debug, PartialEq, Eq)]
	struct Multi {
		a: u8,
		b: String
	}

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	#[unique] // over all values
	#[unique(table)] // multi-column via table
	#[unique(field, table)] // over 2 multi-column values
	struct MultiTable {
		#[unique] // multi-column via field
		field: Multi,
		table: Multi,
		x: u8
	}

	let uniques = [
		"UNIQUE (field_a, field_b)",
		"UNIQUE (table_a, table_b)",
		"UNIQUE (field_a, field_b, table_a, table_b)",
		"UNIQUE (field_a, field_b, table_a, table_b, x)"
	];
	for unique in uniques {
		assert!(
			MultiTable::CREATE_TABLE.contains(unique),
			"definition does not include {unique}:\n{}",
			MultiTable::CREATE_TABLE
		);
	}

	assert!(db.get_all::<MultiTable>()?.is_empty());
	let mt = MultiTable {
		field: Multi { a: 1, b: String::from("abc123") },
		table: Multi { a: 2, b: String::from("def456") },
		x: 123
	};
	assert_eq!(db.insert(&mt).unwrap(), 1);
	db.insert(&mt).expect_err(
		"inserting the same thing should violate unique constraint and fail"
	);
	let different_x = MultiTable {
		x: 124, ..mt
	};
	db.insert(&different_x).expect_err(
		"should violate the unique(field, table) constraint and fail"
	);
	let different_table = MultiTable {
		table: Multi { a: 99, b: String::from("def456") },
		..different_x
	};
	db.insert(&different_table).expect_err(
		"should violate the unique(field) constraint and fail"
	);
	let new_field_a = MultiTable {
		field: Multi { a: 123, b: String::from("abc123") },
		table: Multi { a: 2, b: String::from("def456") },
		x: 123
	};
	db.insert(&new_field_a).expect_err(
		"should violate the unique(table) constraint and fail"
	);
	let new_table_b_also = MultiTable {
		table: Multi { a: 2, b: String::from("NEW def456") },
		..new_field_a
	};
	// now, field AND table are different
	assert_eq!(db.insert(&new_table_b_also).unwrap(), 1);

	Ok(())
}
