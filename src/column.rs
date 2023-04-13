use crate::value::{
	Affinity,
	ForeignKey,
	Check,
	Value
};

pub struct Definition {
	pub name: &'static str,
	pub affinity: Affinity,
	pub unique: bool,
	pub nullable: bool,
	pub reference: Option<ForeignKey>,
	pub checks: Vec<Check>
}


impl Definition {
	pub fn from_value<V: Value>(name: &'static str) -> Definition {
		Self {
			name,
			affinity: V::AFFINITY,
			unique: V::UNIQUE,
			nullable: V::NULLABLE,
			reference: V::FOREIGN_KEY,
			checks: Vec::from(V::CHECKS)
		}
	}

	pub(crate) fn write_sql_to(&self, name: &str, sql: &mut String) {
		sql.push_str(name);
		sql.push(' ');
		if !self.nullable {
			sql.push_str(self.affinity.as_str());
		}
		else {
			sql.push_str(self.affinity.as_str_nullable());
		}
		if let Some(fk_ref) = &self.reference {
			fk_ref.write_sql_to(sql);
		}
		for Check::Sql(check) in self.checks.iter() {
			sql.push_str(" CHECK ( ");
			sql.push_str(name);
			sql.push(' ');
			sql.push_str(check);
			sql.push_str(" ) ");
		}
	}
}
