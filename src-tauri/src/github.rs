use serde::Serialize;
use std::{process::Command, time::Duration};

const QUERY: &str = "query { viewer { login pullRequests(first: 25, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) { nodes { number title url isDraft reviewDecision repository { nameWithOwner } } } } }";

#[derive(Serialize)]
pub struct GitHubStatus {
    pub has_token: bool,
}

#[derive(Serialize)]
pub struct PullRequest {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub review: String,
    pub url: String,
    pub draft: bool,
}

#[derive(Serialize)]
pub struct PullPage {
    pub login: String,
    pub source: String,
    pub pulls: Vec<PullRequest>,
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("engineering-copilot-widget", "github-token").map_err(|e| e.to_string())
}

fn saved_token() -> Option<String> {
    entry().ok().and_then(|e| e.get_password().ok()).filter(|t| !t.trim().is_empty())
}

fn gh_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() { return None; }
    let token = String::from_utf8(output.stdout).ok()?;
    let token = token.trim().to_string();
    if token.is_empty() { None } else { Some(token) }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("engineering-copilot-widget")
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

fn review_label(draft: bool, decision: Option<&str>) -> String {
    if draft { return "Draft".into(); }
    match decision {
        Some("APPROVED") => "Approved".into(),
        Some("CHANGES_REQUESTED") => "Changes requested".into(),
        Some("REVIEW_REQUIRED") => "Review required".into(),
        _ => "No review yet".into(),
    }
}

async fn viewer_pulls(token: &str) -> Result<PullPage, String> {
    let response = client()?.post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .json(&serde_json::json!({ "query": QUERY }))
        .send().await.map_err(|e| format!("Could not reach GitHub: {e}"))?;
    let status = response.status();
    let body: serde_json::Value = response.json().await.map_err(|_| format!("GitHub returned HTTP {status} with an unexpected body"))?;
    if status.as_u16() == 401 { return Err("GitHub rejected this token. Use a personal access token that can read pull requests.".into()); }
    if !status.is_success() { return Err(format!("GitHub returned HTTP {status}")); }
    if let Some(message) = body["errors"].as_array().and_then(|items| items.first()).and_then(|e| e["message"].as_str()) {
        return Err(message.to_string());
    }
    let viewer = &body["data"]["viewer"];
    let login = viewer["login"].as_str().unwrap_or("").to_string();
    if login.is_empty() { return Err("GitHub did not return the signed-in user.".into()); }
    let pulls = viewer["pullRequests"]["nodes"].as_array().into_iter().flatten().filter_map(|node| {
        if node.is_null() { return None; }
        Some(PullRequest {
            repo: node["repository"]["nameWithOwner"].as_str().unwrap_or("").into(),
            number: node["number"].as_u64().unwrap_or(0),
            title: node["title"].as_str().unwrap_or("").into(),
            review: review_label(node["isDraft"].as_bool().unwrap_or(false), node["reviewDecision"].as_str()),
            url: node["url"].as_str().unwrap_or("").into(),
            draft: node["isDraft"].as_bool().unwrap_or(false),
        })
    }).collect();
    Ok(PullPage { login, source: String::new(), pulls })
}

#[tauri::command]
pub fn github_status() -> GitHubStatus {
    GitHubStatus { has_token: saved_token().is_some() }
}

#[tauri::command]
pub async fn save_github_token(token: String) -> Result<String, String> {
    let token = token.trim().to_string();
    if token.is_empty() || token.len() > 300 || token.chars().any(|c| c.is_whitespace()) {
        return Err("Enter a GitHub personal access token with no spaces.".into());
    }
    let page = viewer_pulls(&token).await?;
    entry()?.set_password(&token).map_err(|e| format!("Could not save the token: {e}"))?;
    Ok(page.login)
}

#[tauri::command]
pub fn clear_github_token() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn list_github_pulls() -> Result<PullPage, String> {
    if let Some(token) = saved_token() {
        let mut page = viewer_pulls(&token).await?;
        page.source = "saved".into();
        return Ok(page);
    }
    if let Some(token) = gh_token() {
        let mut page = viewer_pulls(&token).await?;
        page.source = "gh".into();
        return Ok(page);
    }
    Err("Connect GitHub with a personal access token, or sign in with the GitHub CLI.".into())
}
