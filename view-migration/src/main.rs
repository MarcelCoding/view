use sea_orm_migration::cli::run_cli;

use view_migration::Migrator;

#[tokio::main]
async fn main() {
  run_cli(Migrator).await;
}
