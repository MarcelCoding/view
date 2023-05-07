use hex_buffer_serde::{ConstHex, ConstHexForm};
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Client};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use url::Url;

pub(crate) struct ViewClient {
  client: Client,
  base_url: Url,
  token: String,
}

#[derive(Serialize, Clone)]
pub(crate) struct CommitData<'a> {
  description: &'a str,
  files: &'a [FileData],
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct FileData {
  pub(crate) path: String,
  #[serde(with = "ConstHexForm")]
  pub(crate) object_id: [u8; 32],
  pub(crate) fallback: bool,
}

impl ViewClient {
  pub(crate) fn new(base_url: Url, token: String) -> Self {
    Self {
      client: Client::new(),
      base_url: base_url.join("v1").unwrap(),
      token,
    }
  }

  pub(crate) async fn put_commit(
    &self,
    id: &str,
    description: &str,
    files: &[FileData],
  ) -> anyhow::Result<Vec<FileData>> {
    let data = CommitData { description, files };

    let result = self
      .client
      .put(self.base_url.join(&format!("commit/{}", id))?)
      .bearer_auth(&self.token)
      .json(&data)
      .send()
      .await?
      .error_for_status()?
      .json::<Vec<FileData>>()
      .await?;

    Ok(result)
  }

  pub(crate) async fn put_object(&self, id: &str, file: File) -> anyhow::Result<()> {
    let len = file.metadata().await?.len();

    let stream = FramedRead::new(file, BytesCodec::new());

    let part = Part::stream_with_length(Body::wrap_stream(stream), len);
    let form = Form::new().part("file", part);

    self
      .client
      .put(self.base_url.join(&format!("object/{}", id))?)
      .bearer_auth(&self.token)
      .multipart(form)
      .send()
      .await?
      .error_for_status()?;

    Ok(())
  }
}
