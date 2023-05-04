use std::convert::Infallible;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use hyper::header::SERVER;

use hyper::service::Service;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tower::ServiceBuilder;
use tower_http::compression::{Compression, CompressionLayer};

use view_migration::Migrator;
use view_serve::FileService;

pub struct MakeSvc {
  root_dir: PathBuf,
  db: DatabaseConnection,
}

impl<T> Service<T> for MakeSvc {
  type Response = Compression<FileService>;
  type Error = Infallible;
  type Future = Pin<Box<dyn Future<Output=Result<Self::Response, Self::Error>> + Send>>;

  fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Ok(()).into()
  }

  fn call(&mut self, _: T) -> Self::Future {
    let src = FileService {
      root_dir: self.root_dir.clone(),
      db: self.db.clone(),
    };

    let fut = async {
      Ok(ServiceBuilder::new()
        .layer(CompressionLayer::new())
        .service(src))
    };

    Box::pin(fut)
  }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let db =
    Database::connect("postgres://root:Kzb%Nj8PgL2p2~%aWn.B@marcel.hel1.not4y.net:5432/view")
      .await?;

  Migrator::up(&db, None).await?;

  let file_service = MakeSvc {
    root_dir: Path::new("G:\\Work\\IdeaProjects\\view\\data").to_owned(),
    db,
  };

  let service = ServiceBuilder::new()
    .service(file_service);

  let addr = ([127, 0, 0, 1], 8080).into();
  let server = hyper::Server::bind(&addr).serve(service);
  server.await?;

  Ok(())
}
