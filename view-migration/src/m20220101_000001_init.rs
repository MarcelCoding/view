use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Object::Table)
          .col(
            ColumnDef::new(Object::Id)
              .binary()
              .binary_len(265)
              .not_null()
              .primary_key(),
          )
          .col(
            ColumnDef::new(Object::Created)
              .timestamp_with_time_zone()
              .not_null(),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(Commit::Table)
          .col(
            ColumnDef::new(Commit::Id)
              .binary()
              .binary_len(20)
              .not_null()
              .primary_key(),
          )
          .col(ColumnDef::new(Commit::Description).text().not_null())
          .col(
            ColumnDef::new(Commit::Created)
              .timestamp_with_time_zone()
              .not_null(),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(File::Table)
          .col(ColumnDef::new(File::Path).string().not_null())
          .col(
            ColumnDef::new(File::ObjectId)
              .binary()
              .binary_len(256)
              .not_null(),
          )
          .col(
            ColumnDef::new(File::CommitId)
              .binary()
              .binary_len(20)
              .not_null(),
          )
          .primary_key(
            Index::create()
              .col(File::Path)
              .col(File::ObjectId)
              .col(File::CommitId),
          )
          .foreign_key(
            ForeignKey::create()
              .name("FK_file_to_object_id")
              .from(File::Table, File::ObjectId)
              .to(Object::Table, Object::Id),
          )
          .foreign_key(
            ForeignKey::create()
              .name("FK_file_to_commit_id")
              .from(File::Table, File::CommitId)
              .to(Commit::Table, Commit::Id),
          )
          .to_owned(),
      )
      .await?;

    manager
      .create_table(
        Table::create()
          .table(Environment::Table)
          .col(
            ColumnDef::new(Environment::Id)
              .uuid()
              .not_null()
              .primary_key(),
          )
          .col(ColumnDef::new(Environment::Name).string().not_null())
          .col(
            ColumnDef::new(Environment::Domain)
              .string()
              .unique_key()
              .not_null(),
          )
          .col(
            ColumnDef::new(Environment::CommitId)
              .binary()
              .binary_len(20)
              .not_null(),
          )
          .foreign_key(
            ForeignKey::create()
              .name("FK_environment_to_commit_id")
              .from(Environment::Table, Environment::CommitId)
              .to(Commit::Table, Commit::Id),
          )
          .to_owned(),
      )
      .await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(Table::drop().table(Environment::Table).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(File::Table).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(Object::Table).to_owned())
      .await?;

    manager
      .drop_table(Table::drop().table(Commit::Table).to_owned())
      .await
  }
}

#[derive(Iden)]
enum Object {
  Table,
  Id,
  Created,
}

#[derive(Iden)]
enum File {
  Table,
  Path,
  ObjectId,
  CommitId,
}

#[derive(Iden)]
enum Commit {
  Table,
  Id,
  Description,
  Created,
}

#[derive(Iden)]
enum Environment {
  Table,
  Id,
  Name,
  Domain,
  CommitId,
}
