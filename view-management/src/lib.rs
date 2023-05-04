use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};
use time::OffsetDateTime;

use view_entity::{commit, file, object};

pub async fn commit(db: &DatabaseConnection, id: &[u8; 20]) -> anyhow::Result<()> {
  let commit = commit::ActiveModel {
    id: Set(id.to_vec()),
  };

  commit::Entity::insert(commit).exec(db).await?;

  Ok(())
}

pub async fn upload(
  db: &DatabaseConnection,
  commit_id: &[u8; 20],
  object_id: &[u8; 256],
  path: String,
) -> anyhow::Result<bool> {
  let object_is_new = object::Entity::find_by_id(object_id.to_vec())
    .count(db)
    .await?
    == 0;

  if object_is_new {
    let object = object::ActiveModel {
      id: Set(object_id.to_vec()),
      created: Set(OffsetDateTime::now_utc()),
    };

    object::Entity::insert(object).exec(db).await?;
  }

  let file = file::ActiveModel {
    path: Set(path),
    object_id: Set(object_id.to_vec()),
    commit_id: Set(commit_id.to_vec()),
  };

  file::Entity::insert(file).exec(db).await?;

  Ok(object_is_new)
}
