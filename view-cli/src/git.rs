use tokio::process::Command;

pub(crate) async fn get_commit_id() -> anyhow::Result<String> {
  let raw = Command::new("git")
    .arg("rev-parse")
    .arg("HEAD")
    .output()
    .await?
    .stdout;

  Ok(String::from_utf8(raw)?.trim().to_string())
}

pub(crate) async fn get_commit_description(commit_id: &str) -> anyhow::Result<String> {
  let raw = Command::new("git")
    .arg("log")
    .arg("--format=%B")
    .arg("-n")
    .arg("1")
    .arg(commit_id)
    .output()
    .await?
    .stdout;

  Ok(String::from_utf8_lossy(&raw).trim().to_string())
}
