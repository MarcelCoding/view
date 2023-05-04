use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "file")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub path: String,
  #[sea_orm(primary_key)]
  pub object_id: Vec<u8>,
  //[u8; 64],
  #[sea_orm(primary_key)]
  pub commit_id: Vec<u8>, //[u8; 20],
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "super::commit::Entity",
    from = "Column::CommitId",
    to = "super::commit::Column::Id"
  )]
  Commit,
  #[sea_orm(
    belongs_to = "super::object::Entity",
    from = "Column::ObjectId",
    to = "super::object::Column::Id"
  )]
  Object,
}

impl Related<super::commit::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Commit.def()
  }
}

impl Related<super::object::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Object.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
