use sea_orm::prelude::*;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "commit")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: Vec<u8>,
  //[u8; 20],
  #[sea_orm(column_type = "Text")]
  pub description: String,
  pub created: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(has_many = "super::file::Entity")]
  File,
  #[sea_orm(has_many = "super::environment::Entity")]
  Environment,
}

impl Related<super::file::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::File.def()
  }
}

impl Related<super::environment::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Environment.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
