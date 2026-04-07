use std::path::Path;

/// Import a rule bundle into a remote Axiom server.
pub async fn import(bundle_path: &Path, server_url: &str, api_key: &str) -> anyhow::Result<()> {
    let bytes = std::fs::read(bundle_path)?;

    let client = reqwest::Client::new();
    let url    = format!("{server_url}/v1/import");
    let resp   = client.post(&url)
        .header("X-Axiom-Key", api_key)
        .header("Content-Type", "application/yaml")
        .body(bytes)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;
    println!("{}", serde_json::to_string_pretty(&body)?);

    if !status.is_success() {
        anyhow::bail!("server returned {status}");
    }

    Ok(())
}

/// Export all rules from a remote server to a local bundle file.
pub async fn export(server_url: &str, api_key: &str, output: &Path) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let url    = format!("{server_url}/v1/export");
    let resp   = client.get(&url)
        .header("X-Axiom-Key", api_key)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("server returned {status}");
    }

    let body = resp.bytes().await?;
    std::fs::write(output, &body)?;
    println!("Exported to {}", output.display());
    Ok(())
}
