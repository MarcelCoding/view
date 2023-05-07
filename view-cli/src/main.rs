use clap::{Args, Parser};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use url::Url;

use crate::action::Action;

mod action;

#[derive(Parser)]
#[command(version)]
struct Cli {
  #[clap(flatten)]
  general: GeneralArgs,
  #[clap(subcommand)]
  action: Action,
}

#[derive(Args)]
struct GeneralArgs {
  #[clap(short, long, env = "VIEW_URL")]
  url: Url,
  #[clap(short, long, env = "VIEW_TOKEN")]
  token: String,
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

  cli.action.execute(cli.general).await
}
