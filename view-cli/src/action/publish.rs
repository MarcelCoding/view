use clap::Args;

use crate::client::ViewClient;

#[derive(Args)]
pub(crate) struct PublishAction {}

impl PublishAction {
  pub(crate) async fn execute(self, client: ViewClient) -> anyhow::Result<()> {
    Ok(())
  }
}
