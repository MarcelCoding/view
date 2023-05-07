use std::convert::Infallible;
use std::path::PathBuf;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE, HOST, IF_MODIFIED_SINCE, LAST_MODIFIED};
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use mime_guess::Mime;
use sea_orm::{
  ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QuerySelect,
  RelationTrait, Select, SelectTwo,
};
use time::format_description::well_known::Rfc2822;
use time::{OffsetDateTime, UtcOffset};
use tokio::fs::File;
use tokio_util::codec::BytesCodec;
use tokio_util::codec::FramedRead;

use view_entity::{commit, environment, file, object};

fn find_object(domain: &str, path: &str) -> Select<object::Entity> {
  object::Entity::find()
    .join(JoinType::InnerJoin, object::Relation::File.def())
    .join(JoinType::InnerJoin, file::Relation::Commit.def())
    .join(JoinType::InnerJoin, commit::Relation::Environment.def())
    .filter(
      Condition::all()
        .add(file::Column::Path.eq(path))
        .add(environment::Column::Domain.eq(domain)),
    )
}

fn find_fallback_objects(domain: &str) -> SelectTwo<object::Entity, file::Entity> {
  object::Entity::find()
    .find_also_related(file::Entity)
    .join(JoinType::InnerJoin, file::Relation::Commit.def())
    .join(JoinType::InnerJoin, commit::Relation::Environment.def())
    .filter(
      Condition::all()
        .add(file::Column::Fallback.eq(true))
        .add(environment::Column::Domain.eq(domain)),
    )
}

pub struct FileService {
  pub root_dir: PathBuf,
  pub db: DatabaseConnection,
}

impl Service<Request<Body>> for FileService {
  type Response = Response<Body>;
  type Error = Infallible;
  type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

  fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn call(&mut self, req: Request<Body>) -> Self::Future {
    let host = req
      .headers()
      .get(HOST)
      .and_then(|value| value.to_str().ok())
      .and_then(|value| value.split(':').next())
      .unwrap_or("localhost")
      .to_string();

    let db = self.db.clone();
    let root_dir = self.root_dir.clone();

    async move {
      if req.method() != Method::GET {
        return Ok(
          Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .unwrap(),
        );
      }

      let path = req.uri().path();
      let select = find_object(&host, path);
      let object = match select.one(&db).await {
        Ok(Some(object)) => Ok(Some((object, get_mime_type(path)))),
        Ok(None) => {
          let path = if !path.ends_with('/') {
            format!("{}/", path)
          } else {
            path.to_string()
          };

          match find_fallback_objects(&host).all(&db).await {
            Ok(objects) => Ok(
              objects
                .into_iter()
                .flat_map(|(object, file)| {
                  let file = file.unwrap();

                  if let Some(idx) = file.path.rfind('/') {
                    return match &path.strip_prefix(&file.path[..idx + 1]) {
                      Some(remaining) => Some((object, get_mime_type(&file.path), remaining.len())),
                      None => None,
                    };
                  }

                  Some((object, get_mime_type(&file.path), usize::MAX))
                })
                .min_by_key(|(_, _, score)| *score)
                .map(|(object, mime, _)| (object, mime)),
            ),
            Err(err) => Err(err),
          }
        }
        Err(err) => Err(err),
      };

      let response = match object {
        Ok(Some((object, mime))) => {
          if let Some(modified_since) = req
            .headers()
            .get(IF_MODIFIED_SINCE)
            .and_then(|value| String::from_utf8(value.as_bytes().to_vec()).ok())
          {
            match OffsetDateTime::parse(&modified_since, &Rfc2822) {
              Ok(modified_since) => {
                if modified_since >= object.created {
                  return Ok(
                    Response::builder()
                      .status(StatusCode::NOT_MODIFIED)
                      .body(Body::empty())
                      .unwrap(),
                  );
                }
              }
              Err(_) => {
                return Ok(
                  Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::empty())
                    .unwrap(),
                );
              }
            }
          }

          let path = root_dir
            .join(hex::encode(&object.id[..1]))
            .join(hex::encode(&object.id[1..]));

          match File::open(&path).await {
            Ok(file) => {
              let stream = FramedRead::new(file, BytesCodec::new());
              let body = Body::wrap_stream(stream);

              let mut resp = Response::builder()
                .header(CONTENT_TYPE, mime.essence_str())
                .header(LAST_MODIFIED, {
                  let utc_date_time = object
                    .created
                    .to_offset(UtcOffset::UTC)
                    .format(&Rfc2822)
                    .unwrap();
                  let utc_date_time = &utc_date_time[..utc_date_time.len() - 5];
                  format!("{}GMT", utc_date_time)
                });

              if let Some(size) = object.size {
                resp = resp.header(CONTENT_LENGTH, size);
              }

              resp.body(body).unwrap()
            }
            Err(err) => {
              eprint!("Error: {:?}", err);
              Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
            }
          }
        }
        Ok(None) => Response::builder()
          .status(StatusCode::NOT_FOUND)
          .body(Body::empty())
          .unwrap(),
        Err(err) => {
          eprint!("Error: {:?}", err);
          Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap()
        }
      };

      Ok(response)
    }
    .boxed()
  }
}

fn get_mime_type(path: &str) -> Mime {
  mime_guess::from_path(path).first_or_octet_stream()
}
