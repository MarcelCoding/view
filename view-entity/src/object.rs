use sea_orm::prelude::*;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "object")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: Vec<u8>,
  //Vec<u8>, //[u8; 64],
  pub size: Option<i64>, // postgres does not support u64
  pub created: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(has_many = "super::file::Entity")]
  File,
}

impl Related<super::file::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::File.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
