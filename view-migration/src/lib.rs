use sea_orm_migration::async_trait::async_trait;
use sea_orm_migration::{MigrationTrait, MigratorTrait};

mod m20220101_000001_init;

pub struct Migrator;

#[async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![Box::new(m20220101_000001_init::Migration)]
  }
}
