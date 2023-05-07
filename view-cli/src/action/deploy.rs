use std::path::PathBuf;

use clap::Args;
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{info, warn};

use crate::client::{FileData, ViewClient};
use crate::git::{get_commit_description, get_commit_id};

#[derive(Args)]
pub(crate) struct DeployAction {
  #[clap(env = "VIEW_UPLOAD_DIR")]
  upload_dir: PathBuf,
  #[clap(short, long, env = "VIEW_FALLBACK_FILE")]
  fallback: Vec<String>,
}

impl DeployAction {
  pub(crate) async fn execute(self, client: ViewClient) -> anyhow::Result<()> {
    let paths = find_files(self.upload_dir.clone()).await?;
    info!("Found {} files to upload", paths.len());

    let mut files = Vec::with_capacity(paths.len());

    for path in paths {
      let mut hasher = Sha256::new();
      let mut file = File::open(&path).await?;

      let mut buf = [0u8; 4096];
      loop {
        let read = file.read(&mut buf).await?;
        if read == 0 {
          break;
        }

        hasher.update(&buf[..read]);
      }

      let path = path.strip_prefix(&self.upload_dir)?;
      let mut buf = String::new();
      for component in path.components() {
        buf.push('/');
        buf.push_str(&urlencoding::encode(
          &component.as_os_str().to_string_lossy(),
        ));
      }

      files.push(FileData {
        fallback: self.fallback.contains(&buf),
        path: buf,
        object_id: hasher.finalize().into(),
      });
    }

    let commit_id = get_commit_id().await?;
    info!("Publishing as commit {}...", commit_id);

    let commit_description = get_commit_description(&commit_id).await?;

    let objects_to_upload = client
      .put_commit(&commit_id, &commit_description, &files)
      .await?;

    for FileData { object_id, .. } in objects_to_upload {
      if let Some(object) = files.iter().find(|object| object.object_id == object_id) {
        info!("Uploading {}...", object.path);

        let file = File::open(
          self
            .upload_dir
            .join(&*urlencoding::decode(&object.path[1..])?),
        )
        .await?;

        client.put_object(&hex::encode(object_id), file).await?;
      }
    }

    Ok(())
  }
}

async fn find_files(root: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
  let mut out = Vec::new();
  let mut to_visit = vec![root];

  while let Some(dir) = to_visit.pop() {
    info!(
      "Discovering directory {}... ({} remaining)",
      dir.display(),
      to_visit.len()
    );
    let mut read_dir = tokio::fs::read_dir(dir).await?;

    while let Some(entry) = read_dir.next_entry().await? {
      let metadata = entry.metadata().await?;
      let path = entry.path();

      if metadata.is_dir() {
        to_visit.push(path);
      } else if metadata.is_file() {
        out.push(path);
      } else if metadata.is_symlink() {
        warn!("Skipping symlink {}", path.display());
      } else {
        warn!("Unknown file {}", path.display());
      }
    }
  }

  Ok(out)
}
