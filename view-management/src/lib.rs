use std::path::PathBuf;

use anyhow::anyhow;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::{put, IntoMakeService};
use axum::{debug_handler, Json, Router};
use hex::FromHex;
use hex_buffer_serde::{ConstHex, ConstHexForm};
use sea_orm::ActiveValue::Set;
use sea_orm::{
  DatabaseConnection, DatabaseTransaction, EntityTrait, PaginatorTrait, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use view_entity::{commit, file, object};

#[derive(Clone)]
pub struct ManagementState {
  pub db: DatabaseConnection,
  pub root_dir: PathBuf,
}

pub fn router(state: ManagementState) -> IntoMakeService<Router<(), Body>> {
  Router::new()
    .route("/v1/commit/:id", put(commit))
    .route("/v1/object/:id", put(object))
    .with_state(state)
    .into_make_service()
}

#[derive(Deserialize, Clone)]
struct CommitData {
  description: String,
  files: Vec<FileData>,
}

#[derive(Deserialize, Serialize, Clone)]
struct FileData {
  path: String,
  #[serde(with = "ConstHexForm")]
  object_id: [u8; 32],
}

#[debug_handler]
async fn commit(
  State(state): State<ManagementState>,
  Path(id): Path<String>,
  Json(commit): Json<CommitData>,
) -> Result<Json<Vec<FileData>>, StatusCode> {
  let result = match state.db.begin().await {
    Ok(tx) => match commit_endpoint(&tx, id, commit).await {
      Ok(result) => {
        tx.commit().await.unwrap();
        Ok(result)
      }
      Err(err) => Err(err),
    },
    Err(err) => Err::<_, anyhow::Error>(err.into()),
  };

  match result {
    Ok(result) => Ok(Json(result)),
    Err(err) => {
      eprint!("Error: {:?}", err);
      Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
  }
}

async fn commit_endpoint(
  tx: &DatabaseTransaction,
  input_id: String,
  commit_data: CommitData,
) -> anyhow::Result<Vec<FileData>> {
  let id = <[u8; 20]>::from_hex(input_id)?;

  let commit = commit::ActiveModel {
    id: Set(id.to_vec()),
    description: Set(commit_data.description),
    created: Set(OffsetDateTime::now_utc()),
  };

  commit::Entity::insert(commit).exec(tx).await?;

  let mut objects_to_upload = Vec::new();

  for file in commit_data.files {
    let object_is_new = object::Entity::find_by_id(file.object_id.to_vec())
      .count(tx)
      .await?
      == 0;

    if object_is_new {
      let object = object::ActiveModel {
        id: Set(file.object_id.to_vec()),
        created: Set(OffsetDateTime::now_utc()),
      };

      object::Entity::insert(object).exec(tx).await?;
      objects_to_upload.push(file.clone());
    }

    let file = file::ActiveModel {
      path: Set(file.path),
      object_id: Set(file.object_id.to_vec()),
      commit_id: Set(id.to_vec()),
    };

    file::Entity::insert(file).exec(tx).await?;
  }

  Ok(objects_to_upload)
}

#[debug_handler]
async fn object(
  State(state): State<ManagementState>,
  Path(id): Path<String>,
  multipart: Multipart,
) -> Result<(), StatusCode> {
  object_endpoint(state.db, state.root_dir, id, multipart)
    .await
    .map_err(|err| {
      eprint!("Error: {:?}", err);
      StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn object_endpoint(
  db: DatabaseConnection,
  root_dir: PathBuf,
  input_id: String,
  mut multipart: Multipart,
) -> anyhow::Result<()> {
  if input_id.len() != 64 {
    return Err(anyhow!("Expected 64 hex character commit id"));
  }

  let id = <[u8; 32]>::from_hex(&input_id)?;

  if object::Entity::find_by_id(id).count(&db).await? == 0 {
    return Err(anyhow!("object not found"));
  }

  let path = root_dir.join(&input_id[0..2]).join(&input_id[2..]);
  if let Some(parent) = path.parent() {
    if !tokio::fs::try_exists(parent).await? {
      tokio::fs::create_dir(parent).await?;
    }
  }

  let mut file = File::create(path).await?;

  let mut field = multipart
    .next_field()
    .await?
    .ok_or_else(|| anyhow!("Expected file to upload"))?;

  while let Some(chunk) = field.chunk().await? {
    file.write_all(chunk.as_ref()).await?;
  }

  Ok(())
}
