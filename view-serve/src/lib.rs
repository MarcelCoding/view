use std::convert::Infallible;
use std::fmt::Write;
use std::path::PathBuf;
use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use hyper::header::{CONTENT_TYPE, HOST, IF_MODIFIED_SINCE, LAST_MODIFIED};
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use sea_orm::{
  ColumnTrait, Condition, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QuerySelect,
  RelationTrait, Select,
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
      let object = select.one(&db).await;

      let response = match object {
        Ok(Some(object)) => {
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
                )
              }
            }
          }

          let mut buf = String::with_capacity(object.id.len() * 2);
          let mut first = true;

          for x in object.id {
            write!(&mut buf, "{:0>2x}", x).unwrap();
            if first {
              write!(&mut buf, "\\").unwrap();
              first = false;
            }
          }

          let mime = mime_guess::from_path(path).first_or_octet_stream();
          let path = root_dir.join(buf);
          match File::open(&path).await {
            Ok(file) => {
              let stream = FramedRead::new(file, BytesCodec::new());
              let body = Body::wrap_stream(stream);

              Response::builder()
                .header(CONTENT_TYPE, mime.essence_str())
                .header(LAST_MODIFIED, {
                  let utc_date_time = object
                    .created
                    .to_offset(UtcOffset::UTC)
                    .format(&Rfc2822)
                    .unwrap();
                  let utc_date_time = &utc_date_time[..utc_date_time.len() - 5];
                  format!("{}GMT", utc_date_time)
                })
                .body(body)
                .unwrap()
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
