use sea_orm::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "environment")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: Uuid,
  pub name: String,
  pub domain: String,
  pub commit_id: Vec<u8>, // [u8; 20]
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "super::commit::Entity",
    from = "Column::CommitId",
    to = "super::commit::Column::Id"
  )]
  Commit,
}

impl Related<super::commit::Entity> for Entity {
  fn to() -> RelationDef {
    Relation::Commit.def()
  }
}

impl ActiveModelBehavior for ActiveModel {}
