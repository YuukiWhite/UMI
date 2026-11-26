use crate::state::AppEvent;
use futures_util::StreamExt;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MessageStream};

/// Passively observe desktop notifications using D-Bus BecomeMonitor without conflicting with plasmashell.
pub async fn run_notification_monitor(tx: async_channel::Sender<AppEvent>) {
    // Dedicated session connection required because BecomeMonitor puts the connection in monitor-only mode
    let conn = match Connection::session().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("Notification monitor skipped: {err}");
            return;
        }
    };

    let rules = vec!["type='method_call',interface='org.freedesktop.Notifications',member='Notify'"];
    if let Err(err) = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus.Monitoring"),
            "BecomeMonitor",
            &(rules, 0u32),
        )
        .await
    {
        tracing::warn!("D-Bus BecomeMonitor unavailable ({err}); notification peek disabled");
        return;
    }

    tracing::info!("Passive D-Bus notification monitor active (non-conflicting with plasmashell)");
    let mut stream = MessageStream::from(&conn);

    while let Some(Ok(msg)) = stream.next().await {
        if msg.message_type() != zbus::message::Type::MethodCall {
            continue;
        }
        let body = msg.body();
        let Ok((app_name, _replaces_id, _app_icon, summary, notif_body, _actions, _hints, _timeout)) =
            body.deserialize::<(
                String,
                u32,
                String,
                String,
                String,
                Vec<String>,
                HashMap<String, OwnedValue>,
                i32,
            )>()
        else {
            continue;
        };

        // Ignore our own app notifications if any
        if app_name.eq_ignore_ascii_case("umi") {
            continue;
        }

        let app = if app_name.trim().is_empty() {
            "Notification".to_string()
        } else {
            app_name.trim().to_string()
        };
        let sender = if summary.trim().is_empty() {
            app.clone()
        } else {
            summary.trim().to_string()
        };
        let clean_body = strip_basic_html(notif_body.trim());

        let _ = tx
            .send(AppEvent::NotificationReceived {
                app,
                sender,
                body: clean_body,
            })
            .await;
    }
}

fn strip_basic_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '\n' | '\r' if !in_tag => out.push(' '),
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
