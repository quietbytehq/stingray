//! OneDrive libraries. A work or school account can hold several document libraries; rclone's
//! setup takes the first one it is given, which is not always the person's own OneDrive. These
//! helpers ask Microsoft Graph which libraries exist and which one is the main OneDrive, using the
//! connection's own sign-in. The token never leaves this process.

use super::store;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::RcClient;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

const GRAPH: &str = "https://graph.microsoft.com/v1.0";
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub id: String,
    pub name: String,
    /// The person's own OneDrive ("My files").
    pub main: bool,
    /// The library this connection uses now.
    pub current: bool,
}

#[derive(Deserialize)]
struct GraphDrive {
    id: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct GraphList {
    value: Vec<GraphDrive>,
}

/// Reads the access token from rclone's stored token JSON.
pub fn access_token(token_json: &str) -> Option<String> {
    serde_json::from_str::<Value>(token_json)
        .ok()?
        .get("access_token")?
        .as_str()
        .map(str::to_owned)
}

/// Libraries that are internal to Microsoft and never worth offering.
fn hidden(name: &str) -> bool {
    matches!(name, "PersonalCacheLibrary" | "Preservation Hold Library")
}

async fn graph<T: for<'de> Deserialize<'de>>(token: &str, path: &str) -> AppResult<T> {
    let http = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|err| AppError::internal(err.to_string()))?;
    let response = http
        .get(format!("{GRAPH}/{path}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|err| AppError::new(ErrorKind::Rclone, "Stingray couldn't reach Microsoft to list your OneDrive libraries.").with_details(err.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::new(
            ErrorKind::Rclone,
            "Microsoft didn't share the list of libraries. Try signing in again.",
        )
        .with_details(format!("HTTP {}", status.as_u16())));
    }
    response
        .json::<T>()
        .await
        .map_err(|err| AppError::internal(err.to_string()))
}

/// Uses the connection once so rclone refreshes an expired token, then returns the token.
async fn fresh_token(client: &RcClient, name: &str) -> AppResult<String> {
    let _ = client
        .call_value_with_timeout("operations/about", &json!({ "fs": format!("{name}:") }), TIMEOUT)
        .await;
    let raw = store::raw_config(client, name).await?;
    raw.get("token")
        .and_then(|t| access_token(t))
        .ok_or_else(|| AppError::new(ErrorKind::Rclone, "This OneDrive isn't signed in. Sign in again first."))
}

pub async fn libraries(client: &RcClient, name: &str) -> AppResult<Vec<Library>> {
    let current = store::raw_config(client, name)
        .await?
        .get("drive_id")
        .cloned()
        .unwrap_or_default();
    let token = fresh_token(client, name).await?;
    let main: GraphDrive = graph(&token, "me/drive?$select=id,name").await?;
    let all: GraphList = graph(&token, "me/drives?$select=id,name").await?;
    let mut out: Vec<Library> = all
        .value
        .into_iter()
        .filter(|d| !hidden(d.name.as_deref().unwrap_or_default()))
        .map(|d| Library {
            main: d.id == main.id,
            current: d.id == current,
            name: d.name.unwrap_or_else(|| d.id.clone()),
            id: d.id,
        })
        .collect();
    out.sort_by_key(|l| (!l.main, l.name.to_lowercase()));
    Ok(out)
}

pub async fn set_library(client: &RcClient, name: &str, id: &str) -> AppResult<()> {
    if id.trim().is_empty() || id.contains(char::is_whitespace) {
        return Err(AppError::invalid("That library isn't valid."));
    }
    client
        .call_value(
            "config/update",
            &json!({ "name": name, "parameters": { "drive_id": id }, "opt": { "nonInteractive": true } }),
        )
        .await
        .map(|_| ())
}

/// Points a OneDrive connection at the person's own OneDrive. Called right after sign-in.
pub async fn use_main_library(client: &RcClient, name: &str) -> AppResult<()> {
    let token = fresh_token(client, name).await?;
    let main: GraphDrive = graph(&token, "me/drive?$select=id,name").await?;
    set_library(client, name, &main.id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_access_token() {
        assert_eq!(access_token(r#"{"access_token":"abc","expiry":"x"}"#), Some("abc".into()));
        assert_eq!(access_token("not json"), None);
    }

    #[test]
    fn hides_internal_libraries() {
        assert!(hidden("PersonalCacheLibrary"));
        assert!(!hidden("OneDrive - AKESP"));
    }
}
