//! Reads and writes rclone's remote config through the RC API.

use super::providers;
use super::{ConnectionTest, RemoteForm, RemoteSummary};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::RcClient;
use crate::text;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::time::Duration;

/// How long a connection test may take before we call it a failure.
pub const TEST_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Deserialize)]
struct RemoteNames {
    #[serde(default)]
    remotes: Vec<String>,
}

/// Lists every remote in rclone's config, with the wizard it belongs to.
pub async fn list(client: &RcClient) -> AppResult<Vec<RemoteSummary>> {
    let names: RemoteNames = client.call("config/listremotes", &json!({})).await?;
    let mut summaries = Vec::with_capacity(names.remotes.len());
    for name in names.remotes {
        let raw = get_raw(client, &name).await?;
        let rclone_type = raw.get("type").cloned().unwrap_or_default();
        let provider = providers::for_rclone_type(&rclone_type);
        summaries.push(RemoteSummary {
            provider_id: provider.map(|p| p.id.to_owned()),
            provider_title: provider
                .map(|p| p.title.to_owned())
                .unwrap_or_else(|| rclone_type.clone()),
            rclone_type,
            health: super::health::RemoteHealth::Unknown,
            start_path: provider
                .map(|p| providers::start_path(p, &raw))
                .unwrap_or_default(),
            name,
        });
    }
    summaries.sort_by_key(|remote| remote.name.to_lowercase());
    Ok(summaries)
}

pub async fn exists(client: &RcClient, name: &str) -> AppResult<bool> {
    let names: RemoteNames = client.call("config/listremotes", &json!({})).await?;
    Ok(names.remotes.iter().any(|existing| existing == name))
}

/// Returns every key of a remote's section, secrets included. Internal use only.
/// The connection's stored settings, including its sign-in token. Never send this to the UI.
pub async fn raw_config(client: &RcClient, name: &str) -> AppResult<BTreeMap<String, String>> {
    get_raw(client, name).await
}

async fn get_raw(client: &RcClient, name: &str) -> AppResult<BTreeMap<String, String>> {
    let value: Value = client.call("config/get", &json!({ "name": name })).await?;
    let object = value.as_object().cloned().unwrap_or_default();
    Ok(object
        .into_iter()
        .filter_map(|(key, value)| value.as_str().map(|text| (key, text.to_owned())))
        .collect())
}

/// Returns a remote's settings for the edit form. Secret values are withheld.
pub async fn form(client: &RcClient, name: &str) -> AppResult<RemoteForm> {
    let raw = get_raw(client, name).await?;
    let rclone_type = raw
        .get("type")
        .cloned()
        .ok_or_else(|| AppError::new(ErrorKind::InvalidInput, text::remote_missing(name)))?;
    let provider = providers::for_rclone_type(&rclone_type).ok_or_else(|| {
        AppError::new(ErrorKind::Unsupported, text::CANNOT_EDIT_KIND)
            .with_details(format!("rclone type {rclone_type}"))
    })?;

    let secret_keys: Vec<&str> = providers::secret_keys(provider).collect();
    let mut values = BTreeMap::new();
    let mut saved_secrets = Vec::new();
    for (key, value) in raw.iter().filter(|(key, _)| key.as_str() != "type") {
        if secret_keys.contains(&key.as_str()) {
            saved_secrets.push(key.clone());
        } else {
            values.insert(key.clone(), value.clone());
        }
    }
    Ok(RemoteForm {
        name: name.to_owned(),
        provider_id: provider.id.to_owned(),
        values,
        saved_secrets,
    })
}

/// Creates a remote that is configured entirely by parameters (no browser sign-in).
pub async fn create(
    client: &RcClient,
    name: &str,
    rclone_type: &str,
    params: &BTreeMap<String, String>,
) -> AppResult<()> {
    client
        .call_value(
            "config/create",
            &json!({
                "name": name,
                "type": rclone_type,
                "parameters": params,
                "opt": { "nonInteractive": true, "obscure": true },
            }),
        )
        .await
        .map(|_| ())
}

/// Changes the given parameters on an existing remote. Keys not listed keep their values.
pub async fn update(
    client: &RcClient,
    name: &str,
    params: &BTreeMap<String, String>,
) -> AppResult<()> {
    client
        .call_value(
            "config/update",
            &json!({
                "name": name,
                "parameters": params,
                "opt": { "nonInteractive": true, "obscure": true },
            }),
        )
        .await
        .map(|_| ())
}

pub async fn delete(client: &RcClient, name: &str) -> AppResult<()> {
    client
        .call_value("config/delete", &json!({ "name": name }))
        .await
        .map(|_| ())
}

/// Renames by copying the section under the new name and then deleting the old one.
/// Values are copied as stored (already obscured), so they are not obscured twice.
pub async fn rename(client: &RcClient, from: &str, to: &str) -> AppResult<()> {
    let mut raw = get_raw(client, from).await?;
    let rclone_type = raw
        .remove("type")
        .ok_or_else(|| AppError::invalid(text::remote_missing(from)))?;
    let params: Map<String, Value> = raw
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    client
        .call_value(
            "config/create",
            &json!({
                "name": to,
                "type": rclone_type,
                "parameters": params,
                "opt": { "nonInteractive": true, "noObscure": true },
            }),
        )
        .await?;
    delete(client, from).await
}

/// What Stingray can tell about the account behind a connection, for the Accounts screen.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// The signed-in email or user name, when the service reports it.
    pub user: Option<String>,
    /// OneDrive only: "personal", "business" or "documentLibrary" (SharePoint).
    pub drive_type: Option<String>,
    pub used_bytes: Option<i64>,
    pub total_bytes: Option<i64>,
}

/// Reads the email from `rclone config userinfo` output: the line "Email: …" or "Login: …".
pub fn user_from_userinfo(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        ((key == "email" || key == "login" || key == "name") && !value.is_empty())
            .then(|| value.to_owned())
    })
}

/// Collects account details. Each part is optional: a service that can't report it leaves it empty.
pub async fn account_info(client: &RcClient, name: &str) -> AppResult<AccountInfo> {
    let raw = get_raw(client, name).await?;
    let mut info = AccountInfo {
        drive_type: raw.get("drive_type").cloned().filter(|t| !t.is_empty()),
        ..Default::default()
    };
    if let Ok(about) = client
        .call_value_with_timeout("operations/about", &json!({ "fs": format!("{name}:") }), TEST_TIMEOUT)
        .await
    {
        info.used_bytes = about.get("used").and_then(|v| v.as_i64());
        info.total_bytes = about.get("total").and_then(|v| v.as_i64());
    }
    if let Ok(out) = client
        .call_value_with_timeout(
            "core/command",
            &json!({ "command": "config", "arg": ["userinfo", format!("{name}:")], "returnType": "COMBINED_OUTPUT" }),
            TEST_TIMEOUT,
        )
        .await
    {
        let text = out.get("result").and_then(|v| v.as_str()).unwrap_or_default();
        info.user = user_from_userinfo(text);
    }
    Ok(info)
}

/// Checks that a remote can be listed. Uses the remote root (or the network share's folder),
/// which is cheap on every backend.
pub async fn test(client: &RcClient, name: &str) -> AppResult<ConnectionTest> {
    let raw = get_raw(client, name).await?;
    let start = raw
        .get("type")
        .and_then(|t| providers::for_rclone_type(t))
        .map(|p| providers::start_path(p, &raw))
        .unwrap_or_default();
    let result = client
        .call_value_with_timeout(
            "operations/list",
            &json!({ "fs": format!("{name}:"), "remote": start }),
            TEST_TIMEOUT,
        )
        .await;
    Ok(match result {
        Ok(_) => ConnectionTest {
            ok: true,
            slow: false,
            message: text::TEST_OK.to_owned(),
            details: None,
        },
        Err(err) if err.kind == ErrorKind::NotRunning => return Err(err),
        Err(err) if err.kind == ErrorKind::Timeout => ConnectionTest {
            ok: false,
            slow: true,
            message: text::ENGINE_SLOW.to_owned(),
            details: err.details,
        },
        Err(err) => {
            // The engine's raw text was already moved into details by the client.
            let raw = err.details.unwrap_or(err.message);
            ConnectionTest {
                ok: false,
                slow: false,
                message: text::connection_failed(&raw).to_owned(),
                details: Some(raw),
            }
        }
    })
}

#[cfg(test)]
mod account_tests {
    use super::user_from_userinfo;

    #[test]
    fn reads_the_email() {
        assert_eq!(user_from_userinfo("Id: 123\nEmail: ana@example.com\n"), Some("ana@example.com".into()));
        assert_eq!(user_from_userinfo("Login: ana\n"), Some("ana".into()));
        assert_eq!(user_from_userinfo("nothing here"), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_failures_get_a_plain_message_and_keep_the_detail() {
        assert!(
            text::connection_failed("dial tcp: lookup nope.example: no such host")
                .contains("find that server")
        );
        assert!(text::connection_failed("something odd happened").contains("could not reach"));
    }
}
