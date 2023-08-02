use rusqlite::types::{
	ToSql,
	ToSqlOutput,
	FromSql,
	FromSqlResult,
	ValueRef
};

use liter::{
	Table,
	HasKey,
	Id,
	Ref,
	Column,
	Value,
	bind::ToSql2,
	fetch::FromSql2,
	database,
};
use liter::value::{
	ValueDef,
	InnerValueDef,
};


#[test]
fn unique_column() -> rusqlite::Result<()> {
	#[derive(Copy, Clone, Debug, PartialEq, Eq)]
	struct UniqueNumber(u8);
	impl Value for UniqueNumber {
		const DEFINITION: ValueDef = ValueDef {
			unique: true,
			inner: InnerValueDef::Column(<u8 as Column>::DEFINITION),
			reference: None,
			checks: &[]
		};
		type References = ();
	}
	impl ToSql for UniqueNumber {
		fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
			self.0.to_sql()
		}
	}
	impl FromSql for UniqueNumber {
		fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
			u8::column_result(value).map(Self)
		}
	}
	impl ToSql2 for UniqueNumber {}
	impl FromSql2 for UniqueNumber {}


	#[database]
	struct Db (Item);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Item {
		number: UniqueNumber
	}
	assert!(
		Item::CREATE_TABLE.contains("UNIQUE"),
		"definition must include UNIQUE constraint"
	);

	let db = Db::create_in_memory()?;
	assert!(db.get_all::<Item>()?.is_empty());
	let item = Item {number: UniqueNumber(9)};
	assert_eq!(db.insert(&item).unwrap(), 1);

	let item_2 = item.clone();
	db.insert(&item_2).expect_err(
		"inserting the same number should violate unique constraint and fail"
	);

	Ok(())
}

#[test]
fn foreign_key() -> rusqlite::Result<()> {
	#[database]
	struct Db (File, Block, Access);

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct File {
		#[key]
		id: Id,
		name: String,
		permissions: u16
	}

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Block {
		#[key]
		indx: u64,
		#[key]
		file: Ref<File>,
		data: Vec<u8>
	}

	#[derive(Table, Clone, Debug, PartialEq, Eq)]
	struct Access {
		block: Ref<Block>,
		timestamp: u64
	}

	let db = Db::create_in_memory()?;

	assert!(db.get_all::<File>()?.is_empty());
	assert!(db.get_all::<Block>()?.is_empty());
	assert!(db.get_all::<Access>()?.is_empty());

	// Create File, Block & Access
	let mut file = File {
		id: Id::NULL,
		name: "test.txt".to_string(),
		permissions: 0o777
	};
	db.create(&mut file).unwrap();

	let block = Block {
		indx: 0,
		file: Ref::make_ref(&file),
		data: Vec::from(&b"Test123 :^)"[..])
	};
	db.insert(&block).unwrap();

	let access = Access {
		block: Ref::make_ref(&block),
		timestamp: 1690451279
	};
	db.insert(&access).unwrap();

	// Try to create Block for non-existent File
	let block_2 = Block {
		indx: 0,
		// invalid reference
		file: Ref(Id::from_i64(10_000)),
		data: vec![]
	};
	let err = db.insert(&block_2).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	// Try to create Access for non-existent File, index, & Block
	let access_2 = Access {
		// index exists, File doesn'T
		block: Ref((0, Ref(Id::from_i64(10_000)))),
		timestamp: 1690451279
	};
	let err = db.insert(&access_2).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	let access_3 = Access {
		// File exists, index doesn't
		block: Ref((10_000, block.file.clone())),
		timestamp: 1690451279
	};
	let err = db.insert(&access_3).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	let access_4 = Access {
		// neither Index nor File exist
		block: Ref((10_000, Ref(Id::from_i64(10_000)))),
		timestamp: 1690451279
	};
	let err = db.insert(&access_4).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	// Try to delete File that still has a Block
	let err = db.execute("DELETE FROM file WHERE id = ?", &file.id).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	// Try to delete the Block that still has an Access
	let err = db.execute(
		"DELETE FROM block WHERE indx = ? AND file = ?",
		&block.get_key()
	).unwrap_err();
	assert!(err.to_string().contains("FOREIGN KEY constraint"));

	// Delete the Access, then delete the Block, then delete the File
	assert_eq!(
		db.execute(
			"DELETE FROM access WHERE block_indx = ? AND block_file = ?",
			&access.block
		).unwrap(),
		1
	);
	assert_eq!(
		db.execute("DELETE FROM block WHERE file = ?", &file.id).unwrap(),
		1
	);
	db.execute("DELETE FROM file WHERE id = ?", &file.id).unwrap();

	Ok(())
}
