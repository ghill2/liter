use liter::{
	//Schema,
	database,
	//Database,
	Id,
	Ref,
	Table
};

#[database]
struct Dictionary (
	Language,
	Word
);

#[derive(Debug, Table)]
struct Language {
	#[key]
	id: Id,
	name: String
}

#[derive(Debug, Table)]
struct Word {
	#[key]
	word: String,
	//#[key]
	language: Ref<Language>,
	definition: String
}

fn main() {
	let _dict = Dictionary::create_in_memory().unwrap();
}
