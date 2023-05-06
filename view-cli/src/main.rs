use std::path::PathBuf;

use clap::Parser;
use hex_buffer_serde::{Hex, HexForm};
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use url::Url;

#[derive(Parser)]
struct Args {
  #[clap(short, long, env = "VIEW_URL")]
  url: Url,
  #[clap(short, long, env = "VIEW_TOKEN")]
  token: String,
  #[clap(env = "VIEW_UPLOAD_DIR")]
  upload_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct FileObj {
  path: String,
  #[serde(with = "HexForm")]
  object_id: [u8; 32],
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();

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

  let paths = find_files(args.upload_dir.clone()).await?;
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

    let path = path.strip_prefix(&args.upload_dir)?;
    let mut buf = String::new();
    for component in path.components() {
      buf.push('/');
      buf.push_str(&urlencoding::encode(
        &component.as_os_str().to_string_lossy(),
      ));
    }

    files.push(FileObj {
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

  let client = Client::new();

  let objects_to_upload = client
    .put(args.url.join(&format!("v1/commit/{}", commit_id))?)
    .bearer_auth(&args.token)
    .json(&files)
    .send()
    .await?
    .error_for_status()?
    .json::<Vec<FileObj>>()
    .await?;

  for FileObj { object_id, .. } in objects_to_upload {
    if let Some(object) = files.iter().find(|object| object.object_id == object_id) {
      info!("Uploading {}...", object.path);

      let file = File::open(
        args
          .upload_dir
          .join(&*urlencoding::decode(&object.path[1..])?),
      )
      .await?;
      let len = file.metadata().await?.len();

      let stream = FramedRead::new(file, BytesCodec::new());

      let part = Part::stream_with_length(Body::wrap_stream(stream), len);
      let form = Form::new().part("file", part);

      let url = args
        .url
        .join(&format!("v1/object/{}", hex::encode(object_id)))?;

      client
        .put(url)
        .bearer_auth(&args.token)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;
    }
  }

  Ok(())
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
