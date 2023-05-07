use std::path::PathBuf;

use crate::GeneralArgs;
use clap::Args;
use hex_buffer_serde::{ConstHex, ConstHexForm};
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{info, warn};

#[derive(Args)]
pub(crate) struct DeployAction {
  #[clap(env = "VIEW_UPLOAD_DIR")]
  upload_dir: PathBuf,
  #[clap(short, long, env = "VIEW_FALLBACK_FILE")]
  fallback: Vec<String>,
}

#[derive(Serialize, Clone)]
struct CommitData<'a> {
  description: String,
  files: &'a [FileData],
}

#[derive(Deserialize, Serialize, Clone)]
struct FileData {
  path: String,
  #[serde(with = "ConstHexForm")]
  object_id: [u8; 32],
  fallback: bool,
}

impl DeployAction {
  pub(crate) async fn execute(self, general: GeneralArgs) -> anyhow::Result<()> {
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

    let commit_id = String::from_utf8(
      Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .await?
        .stdout,
    )?;

    let commit_id = commit_id.trim();
    info!("Publishing as commit {}...", commit_id);

    let commit_description = String::from_utf8_lossy(
      &Command::new("git")
        .arg("log")
        .arg("--format=%B")
        .arg("-n")
        .arg("1")
        .arg(commit_id)
        .output()
        .await?
        .stdout,
    )
    .trim()
    .to_string();

    let client = Client::new();

    let objects_to_upload = client
      .put(general.url.join(&format!("v1/commit/{}", commit_id))?)
      .bearer_auth(&general.token)
      .json(&CommitData {
        description: commit_description,
        files: &files,
      })
      .send()
      .await?
      .error_for_status()?
      .json::<Vec<FileData>>()
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
        let len = file.metadata().await?.len();

        let stream = FramedRead::new(file, BytesCodec::new());

        let part = Part::stream_with_length(Body::wrap_stream(stream), len);
        let form = Form::new().part("file", part);

        let url = general
          .url
          .join(&format!("v1/object/{}", hex::encode(object_id)))?;

        client
          .put(url)
          .bearer_auth(&general.token)
          .multipart(form)
          .send()
          .await?
          .error_for_status()?;
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
