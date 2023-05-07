use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use clap::Parser;
use hyper::service::Service;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tower::ServiceBuilder;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use view_management::{router, ManagementState};
use view_migration::Migrator;
use view_serve::FileService;

pub struct MakeSvc {
  root_dir: PathBuf,
  db: DatabaseConnection,
}

impl<T> Service<T> for MakeSvc {
  type Response = FileService;
  type Error = Infallible;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

  fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    Ok(()).into()
  }

  fn call(&mut self, _: T) -> Self::Future {
    let src = FileService {
      root_dir: self.root_dir.clone(),
      db: self.db.clone(),
    };

    let fut = async {
      Ok(
        ServiceBuilder::new()
          // .layer(CompressionLayer::new())
          .service(src),
      )
    };

    Box::pin(fut)
  }
}

#[derive(Parser)]
struct Args {
  #[clap(short, long, env = "VIEW_DB_URL")]
  db_url: String,
  #[clap(short, long, env = "VIEW_ROOT_DIR")]
  root_dir: PathBuf,
  #[clap(short, long, env = "VIEW_SERVE_ADDR", default_value = "0.0.0.0:8080")]
  serve_addr: SocketAddr,
  #[clap(short, long, env = "VIEW_MGNT_ADDR", default_value = "0.0.0.0:8081")]
  mgnt_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();

  let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::INFO)
    .compact()
    .finish();

  tracing::subscriber::set_global_default(subscriber)?;

  info!(concat!(
    "Booting ",
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    "..."
  ));

  let db = Database::connect(&args.db_url).await?;

  Migrator::up(&db, None).await?;

  let state = ManagementState {
    db: db.clone(),
    root_dir: args.root_dir.clone(),
  };

  tokio::spawn(async move {
    let mgnt = hyper::Server::bind(&args.mgnt_addr).serve(router(state));

    info!("Management is listening on http://{}...", args.mgnt_addr);

    mgnt.await.unwrap();
  });

  let file_service = MakeSvc {
    root_dir: args.root_dir,
    db,
  };

  let service = ServiceBuilder::new().service(file_service);

  let server = hyper::Server::bind(&args.serve_addr).serve(service);
  info!("Serve is listening on http://{}...", args.mgnt_addr);

  server.await?;

  Ok(())
}
