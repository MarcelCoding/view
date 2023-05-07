use crate::GeneralArgs;
use clap::Args;

#[derive(Args)]
pub(crate) struct PublishAction {}

impl PublishAction {
  pub(crate) async fn execute(self, general: GeneralArgs) -> anyhow::Result<()> {
    Ok(())
  }
}
