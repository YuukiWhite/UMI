use crate::state::AppEvent;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{Connection, MatchRule, MessageStream, Proxy};

/// Capture an interactive screen region natively on KDE Wayland via `org.freedesktop.portal.Screenshot`,
/// return the PNG bytes in memory, and delete the temporary portal file.
pub async fn capture_screenshot_png() -> Result<Vec<u8>> {
    let conn = Connection::session().await?;
    let portal = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .await
    .context("XDG Screenshot portal unavailable")?;

    let token = format!("umi_ocr_{}", chrono::Utc::now().timestamp_millis());
    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    opts.insert("handle_token", Value::from(token.as_str()));
    opts.insert("interactive", Value::from(true));

    // Subscribe to portal Request.Response before invoking Screenshot so we never miss the signal
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.portal.Request")?
        .member("Response")?
        .build();
    let mut stream = MessageStream::for_match_rule(rule, &conn, Some(8)).await?;

    let req_path: OwnedObjectPath = portal
        .call("Screenshot", &("", opts))
        .await
        .context("Screenshot portal call failed")?;

    while let Some(Ok(msg)) = stream.next().await {
        let matches_path = msg
            .header()
            .path()
            .map(|p| p.as_str() == req_path.as_str())
            .unwrap_or(false);
        if !matches_path {
            continue;
        }
        let (code, results): (u32, HashMap<String, OwnedValue>) = msg.body().deserialize()?;
        if code != 0 {
            anyhow::bail!("Screenshot cancelled by user (code {code})");
        }
        let uri = results
            .get("uri")
            .and_then(|v| String::try_from(v.clone()).ok())
            .context("Screenshot portal response missing uri")?;
        let path_str = uri
            .strip_prefix("file://")
            .unwrap_or(&uri)
            .replace("%20", " ");
        let path = PathBuf::from(&path_str);
        let bytes = tokio::fs::read(&path)
            .await
            .with_context(|| format!("Failed to read screenshot {}", path.display()))?;
        let _ = tokio::fs::remove_file(&path).await;
        return Ok(bytes);
    }

    anyhow::bail!("Screenshot portal closed without response")
}

/// Register native Wayland global shortcuts (`Ctrl+Alt+Space` for Ask UMI AI, `Ctrl+Alt+T` for Smart OCR)
/// via `org.freedesktop.portal.GlobalShortcuts` on KDE Plasma 6.8+.
pub async fn run_global_shortcuts_portal(tx: async_channel::Sender<AppEvent>) {
    let Ok(conn) = Connection::session().await else {
        return;
    };
    let Ok(portal) = Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
    )
    .await
    else {
        return;
    };

    let session_token = format!("umi_gs_{}", std::process::id());
    let req_token = format!("umi_gs_req_{}", std::process::id());
    let mut create_opts: HashMap<&str, Value<'_>> = HashMap::new();
    create_opts.insert("handle_token", Value::from(req_token.as_str()));
    create_opts.insert("session_handle_token", Value::from(session_token.as_str()));

    let Ok(req_rule) = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.portal.Request")
        .and_then(|b| b.member("Response"))
    else {
        return;
    };
    let Ok(mut req_stream) = MessageStream::for_match_rule(req_rule.build(), &conn, Some(8)).await else {
        return;
    };

    let Ok(req_path) = portal.call::<_, _, OwnedObjectPath>("CreateSession", &(create_opts,)).await else {
        tracing::debug!("GlobalShortcuts portal CreateSession unsupported; use CLI `umi ask` / `umi ocr`");
        return;
    };

    let mut session_handle: Option<OwnedObjectPath> = None;
    while let Some(Ok(msg)) = req_stream.next().await {
        if msg.header().path().map(|p| p.as_str()) != Some(req_path.as_str()) {
            continue;
        }
        if let Ok((code, results)) = msg.body().deserialize::<(u32, HashMap<String, OwnedValue>)>() {
            if code == 0 {
                session_handle = results
                    .get("session_handle")
                    .and_then(|v| {
                        OwnedObjectPath::try_from(v.clone())
                            .ok()
                            .or_else(|| {
                                String::try_from(v.clone())
                                    .ok()
                                    .and_then(|s| OwnedObjectPath::try_from(s).ok())
                            })
                    });
            }
        }
        break;
    }

    let Some(session) = session_handle else {
        return;
    };

    // Bind preferred shortcuts (KDE Plasma stores/allows customizing these in System Settings -> Shortcuts)
    let mut ask_opts: HashMap<&str, Value<'_>> = HashMap::new();
    ask_opts.insert("description", Value::from("Open Ask UMI AI in notch"));
    ask_opts.insert("preferred_trigger", Value::from("CTRL+ALT+space"));

    let mut ocr_opts: HashMap<&str, Value<'_>> = HashMap::new();
    ocr_opts.insert("description", Value::from("Smart Screen OCR (Gemini Vision)"));
    ocr_opts.insert("preferred_trigger", Value::from("CTRL+ALT+t"));

    let mut toggle_opts: HashMap<&str, Value<'_>> = HashMap::new();
    toggle_opts.insert("description", Value::from("Toggle UMI Notch"));
    toggle_opts.insert("preferred_trigger", Value::from("LOGO+n"));

    let shortcuts: Vec<(&str, HashMap<&str, Value<'_>>)> = vec![
        ("ask_umi", ask_opts),
        ("smart_ocr", ocr_opts),
        ("toggle_umi", toggle_opts),
    ];
    let mut bind_opts: HashMap<&str, Value<'_>> = HashMap::new();
    let bind_token = format!("umi_bind_{}", std::process::id());
    bind_opts.insert("handle_token", Value::from(bind_token.as_str()));

    if let Err(err) = portal
        .call::<_, _, OwnedObjectPath>("BindShortcuts", &(session.clone(), shortcuts, "", bind_opts))
        .await
    {
        tracing::debug!("GlobalShortcuts BindShortcuts skipped: {err}");
        return;
    }

    tracing::info!("XDG GlobalShortcuts portal registered (Ctrl+Alt+Space, Ctrl+Alt+T, Meta+N)");

    let Ok(act_rule) = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.portal.GlobalShortcuts")
        .and_then(|b| b.member("Activated"))
    else {
        return;
    };
    let Ok(mut act_stream) = MessageStream::for_match_rule(act_rule.build(), &conn, Some(16)).await else {
        return;
    };

    while let Some(Ok(msg)) = act_stream.next().await {
        let Ok((_sess, shortcut_id, _ts, _opts)) =
            msg.body()
                .deserialize::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>()
        else {
            continue;
        };
        match shortcut_id.as_str() {
            "ask_umi" => {
                let _ = tx.send(AppEvent::OpenAskAi).await;
            }
            "smart_ocr" => {
                let _ = tx.send(AppEvent::TriggerOcr).await;
            }
            "toggle_umi" => {
                let _ = tx.send(AppEvent::ToggleNotch).await;
            }
            _ => {}
        }
    }
}
