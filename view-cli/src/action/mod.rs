use clap::Subcommand;

use crate::action::deploy::DeployAction;
use crate::action::publish::PublishAction;
use crate::GeneralArgs;

mod deploy;
mod publish;

#[derive(Subcommand)]
pub(crate) enum Action {
  Deploy(DeployAction),
  Publish(PublishAction),
}

impl Action {
  pub(crate) async fn execute(self, general: GeneralArgs) -> anyhow::Result<()> {
    match self {
      Action::Deploy(action) => action.execute(general).await,
      Action::Publish(action) => action.execute(general).await,
    }
  }
}
