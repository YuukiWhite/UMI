use anyhow::{Context, Result};
use zbus::{Connection, Proxy};

const WALLET_FOLDER: &str = "umi";
const WALLET_APP: &str = "umi";

/// Key identifiers stored encrypted in KDE Wallet (`org.kde.kwalletd6`).
pub fn key_name_for_provider(provider: &str) -> &'static str {
    match provider.trim().to_ascii_lowercase().as_str() {
        "claude" | "anthropic" => "claude_api_key",
        "openai" | "openrouter" => "openai_api_key",
        _ => "gemini_api_key",
    }
}

fn env_var_for_key(key_name: &str) -> &'static str {
    match key_name {
        "claude_api_key" => "ANTHROPIC_API_KEY",
        "openai_api_key" => "OPENAI_API_KEY",
        _ => "GEMINI_API_KEY",
    }
}

async fn open_kwallet() -> Result<(Proxy<'static>, i32)> {
    let conn = Connection::session().await?;
    let proxy = Proxy::new(
        &conn,
        "org.kde.kwalletd6",
        "/modules/kwalletd6",
        "org.kde.KWallet",
    )
    .await
    .context("Failed to connect to org.kde.kwalletd6")?;

    let wallet_name: String = proxy
        .call("networkWallet", &())
        .await
        .unwrap_or_else(|_| "kdewallet".to_string());
    let handle: i32 = proxy
        .call("open", &(wallet_name.as_str(), 0i64, WALLET_APP))
        .await
        .context("Failed to open KDE Wallet")?;
    if handle < 0 {
        anyhow::bail!("KDE Wallet returned invalid handle ({handle})");
    }
    Ok((proxy, handle))
}

/// Store an encrypted API key inside KDE Wallet (never plaintext in config.toml).
pub async fn save_secret(provider: &str, api_key: &str) -> Result<()> {
    let key_name = key_name_for_provider(provider);
    let (proxy, handle) = open_kwallet().await?;
    let has_folder: bool = proxy
        .call("hasFolder", &(handle, WALLET_FOLDER, WALLET_APP))
        .await
        .unwrap_or(false);
    if !has_folder {
        let _: bool = proxy
            .call("createFolder", &(handle, WALLET_FOLDER, WALLET_APP))
            .await
            .unwrap_or(false);
    }
    let rc: i32 = proxy
        .call(
            "writePassword",
            &(handle, WALLET_FOLDER, key_name, api_key.trim(), WALLET_APP),
        )
        .await
        .context("Failed to write secret to KDE Wallet")?;
    if rc != 0 {
        anyhow::bail!("KDE Wallet writePassword returned code {rc}");
    }
    Ok(())
}

/// Read an API key from KDE Wallet, with fallback to standard environment variables.
pub async fn read_secret(provider: &str) -> Option<String> {
    let key_name = key_name_for_provider(provider);
    if let Ok((proxy, handle)) = open_kwallet().await {
        if let Ok(val) = proxy
            .call::<_, _, String>(
                "readPassword",
                &(handle, WALLET_FOLDER, key_name, WALLET_APP),
            )
            .await
        {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    let env_name = env_var_for_key(key_name);
    std::env::var(env_name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
