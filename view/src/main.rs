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
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tower::ServiceBuilder;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use url::Url;

use view_management::{ManagementState, router};
use view_migration::Migrator;
use view_serve::FileService;

pub struct MakeSvc {
  root_dir: PathBuf,
  db: DatabaseConnection,
}

impl<T> Service<T> for MakeSvc {
  type Response = FileService;
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
struct Cli {
  #[clap(short, long, env = "VIEW_DB_URL")]
  db_url: Url,
  #[clap(short = 'u', long, env = "VIEW_DB_USER", required_unless_present = "db_user_path")]
  db_user: Option<String>,
  #[clap(long, env = "VIEW_DB_USER_PATH", required_unless_present = "db_user")]
  db_user_path: Option<PathBuf>,
  #[clap(short = 'p', long, env = "VIEW_DB_PASS", required_unless_present = "db_pass_path")]
  db_pass: Option<String>,
  #[clap(long, env = "VIEW_DB_PASS_PATH", required_unless_present = "db_pass")]
  db_pass_path: Option<PathBuf>,
  #[clap(short, long, env = "VIEW_ROOT_DIR")]
  root_dir: PathBuf,
  #[clap(short, long, env = "VIEW_SERVE_ADDR", default_value = "0.0.0.0:8080")]
  serve_addr: SocketAddr,
  #[clap(short, long, env = "VIEW_MGNT_ADDR", default_value = "0.0.0.0:8081")]
  mgnt_addr: SocketAddr,
  #[clap(short = 't', long, env = "VIEW_MGNT_TOKEN", required_unless_present = "mgnt_token_path")]
  mgnt_token: Option<String>,
  #[clap(long, env = "VIEW_MGNT_TOKEN_PATH", required_unless_present = "mgnt_token")]
  mgnt_token_path: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let cli = Cli::parse();

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

  let mut db_url = cli.db_url;

  db_url.set_username(&(match cli.db_user_path {
    Some(path) => {
      let mut file = File::open(path).await?;
      let mut buf = String::new();
      file.read_to_string(&mut buf).await?;
      buf
    }
    None => cli.db_user.unwrap(),
  })).expect("DB URL is missing the base (protocol & host)");

  db_url.set_password(Some(&(match cli.db_pass_path {
    Some(path) => {
      let mut file = File::open(path).await?;
      let mut buf = String::new();
      file.read_to_string(&mut buf).await?;
      buf
    }
    None => cli.db_pass.unwrap(),
  }))).expect("DB URL is missing the base (protocol & host)");

  let db = Database::connect(db_url.as_str()).await?;

  Migrator::up(&db, None).await?;

  let state = ManagementState {
    db: db.clone(),
    root_dir: cli.root_dir.clone(),
  };

  let token = match cli.mgnt_token_path {
    Some(path) => {
      let mut file = File::open(path).await?;
      let mut buf = String::new();
      file.read_to_string(&mut buf).await?;
      buf
    }
    None => cli.mgnt_token.unwrap(),
  };

  tokio::spawn(async move {
    let mgnt = hyper::Server::bind(&cli.mgnt_addr).serve(router(state, &token));

    info!("Management is listening on http://{}...", cli.mgnt_addr);

    mgnt.await.unwrap();
  });

  let file_service = MakeSvc {
    root_dir: cli.root_dir,
    db,
  };

  let service = ServiceBuilder::new().service(file_service);

  let server = hyper::Server::bind(&cli.serve_addr).serve(service);
  info!("Serve is listening on http://{}...", cli.mgnt_addr);

  server.await?;

  Ok(())
}
