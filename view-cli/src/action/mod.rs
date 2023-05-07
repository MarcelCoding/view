use clap::Subcommand;

use crate::action::deploy::DeployAction;
use crate::action::publish::PublishAction;
use crate::client::ViewClient;
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
    let client = ViewClient::new(general.url, general.token);

    match self {
      Action::Deploy(action) => action.execute(client).await,
      Action::Publish(action) => action.execute(client).await,
    }
  }
}
