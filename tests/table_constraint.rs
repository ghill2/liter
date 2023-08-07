use liter::{
	Id,
	Table,
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
