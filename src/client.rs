//! HTTP to the Bugsink API, and the translation from HTTP status to [`ErrorCode`].
//!
//! One blocking agent per process: a CLI makes a handful of requests, so an async runtime
//! would cost more in startup than it could save. Connections are pooled by the agent.

use std::time::Duration;

use serde_json::Value;
use ureq::Agent;
use ureq::http::Response;

use crate::error::{Error, ErrorCode, Result};

const API_PATH: &str = "api/canonical/0/";
const MAX_BODY: u64 = 512 * 1024 * 1024;

#[derive(Clone)]
pub struct Client {
    agent: Agent,
    base: String,
    auth: String,
}

enum Method {
    Get,
    Post,
    Patch,
    Delete,
}

impl Client {
    pub fn new(endpoint: &str, api_token: &str) -> Client {
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(100)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .http_status_as_error(false)
            .user_agent(concat!("bugsink-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();

        let mut root = endpoint.trim().trim_end_matches('/').to_string();
        if !root.ends_with(API_PATH.trim_end_matches('/')) {
            root = format!("{root}/{API_PATH}");
        } else if !root.ends_with('/') {
            root.push('/');
        }

        Client { agent, base: root, auth: format!("Bearer {api_token}") }
    }

    pub fn get(&self, path: &str) -> Result<Value> {
        self.send(Method::Get, path, None)
    }

    pub fn get_text(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.base, path.trim_start_matches('/'));
        let res = self
            .agent
            .get(&url)
            .header("Authorization", &self.auth)
            .header("Accept", "text/markdown, text/plain;q=0.9, */*;q=0.8")
            .call()
            .map_err(transport_error)?;

        read_text(res)
    }

    pub fn post_empty(&self, path: &str) -> Result<Value> {
        self.send(Method::Post, path, None)
    }

    pub fn post(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(Method::Post, path, Some(body))
    }

    pub fn patch(&self, path: &str, body: &Value) -> Result<Value> {
        self.send(Method::Patch, path, Some(body))
    }

    pub fn delete(&self, path: &str) -> Result<Value> {
        self.send(Method::Delete, path, None)
    }

    fn send(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value> {
        let url = format!("{}{}", self.base, path.trim_start_matches('/'));

        macro_rules! headers {
            ($req:expr) => {
                $req.header("Authorization", &self.auth).header("Accept", "application/json")
            };
        }

        let result = match (method, body) {
            (Method::Get, _) => headers!(self.agent.get(&url)).call(),
            (Method::Delete, _) => headers!(self.agent.delete(&url)).call(),
            (Method::Post, Some(b)) => {
                let json = serde_json::to_vec(b).expect("a Value always serializes");
                headers!(self.agent.post(&url)).header("Content-Type", "application/json").send(&json[..])
            }
            (Method::Post, None) => headers!(self.agent.post(&url)).send_empty(),
            (Method::Patch, Some(b)) => {
                let json = serde_json::to_vec(b).expect("a Value always serializes");
                headers!(self.agent.patch(&url)).header("Content-Type", "application/json").send(&json[..])
            }
            (Method::Patch, None) => headers!(self.agent.patch(&url)).send_empty(),
        };

        let response = result.map_err(transport_error)?;
        read_json(response)
    }

    /// Walks cursor pagination and returns items up to `limit` (or all if `None`).
    pub fn get_paged<F>(&self, path: &str, limit: Option<usize>, keep: Option<F>) -> Result<Vec<Value>>
    where
        F: Fn(&Value) -> bool,
    {
        let mut results = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let url = match &cursor {
                None => path.to_string(),
                Some(c) => {
                    let sep = if path.contains('?') { '&' } else { '?' };
                    format!("{path}{sep}cursor={}", seg(c))
                }
            };

            let page = match self.get(&url)? {
                Value::Object(map) => map,
                Value::Array(arr) => {
                    for item in arr {
                        if let Some(ref k) = keep
                            && !k(&item)
                        {
                            continue;
                        }
                        results.push(item);
                        if let Some(l) = limit
                            && results.len() >= l
                        {
                            return Ok(results);
                        }
                    }
                    break;
                }
                _ => break,
            };

            if let Some(Value::Array(items)) = page.get("results") {
                for item in items {
                    if let Some(ref k) = keep
                        && !k(item)
                    {
                        continue;
                    }
                    results.push(item.clone());
                    if let Some(l) = limit
                        && results.len() >= l
                    {
                        return Ok(results);
                    }
                }
            }

            cursor = extract_cursor(page.get("next").and_then(Value::as_str));
            if cursor.is_none() {
                break;
            }
        }

        Ok(results)
    }

    /// Accepts a numeric project id, a slug or a project name.
    pub fn resolve_project_id(&self, project: &str) -> Result<i64> {
        if let Ok(id) = project.trim().parse::<i64>() {
            return Ok(id);
        }

        let projects = self.get_paged("projects/", None, None::<fn(&Value) -> bool>)?;
        let mut matches = Vec::new();
        for item in projects {
            if matches_key(&item, "slug", project) || matches_key(&item, "name", project) {
                matches.push(item);
            }
        }

        if matches.is_empty() {
            return Err(Error::invalid(format!("No project found matching '{project}'.")));
        }
        if matches.len() > 1 {
            return Err(Error::invalid(format!(
                "'{project}' matches {} projects. Use the project id instead.",
                matches.len()
            )));
        }

        matches[0]
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| Error::invalid(format!("Project '{project}' has no valid id.")))
    }

    /// Accepts a team UUID or a team name.
    pub fn resolve_team_id(&self, team: &str) -> Result<String> {
        let team = team.trim();
        // If it looks like a UUID (e.g. 36 chars with hyphens), return as is
        if team.len() == 36 && team.chars().filter(|c| *c == '-').count() == 4 {
            return Ok(team.to_string());
        }

        let teams = self.get_paged("teams/", None, None::<fn(&Value) -> bool>)?;
        let mut matches = Vec::new();
        for item in teams {
            if matches_key(&item, "id", team) || matches_key(&item, "name", team) {
                matches.push(item);
            }
        }

        if matches.is_empty() {
            return Err(Error::invalid(format!("No team found named '{team}'.")));
        }
        if matches.len() > 1 {
            return Err(Error::invalid(format!(
                "'{team}' matches {} teams. Use the team UUID instead.",
                matches.len()
            )));
        }

        matches[0]
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::invalid(format!("Team '{team}' has no valid id.")))
    }

    /// Returns the UUID of the most recent event on an issue, for stacktrace lookups.
    pub fn resolve_latest_event_id(&self, issue: &str) -> Result<String> {
        let path = format!("events/?issue={}&order=desc", seg(issue));
        let events = self.get_paged(&path, Some(1), None::<fn(&Value) -> bool>)?;
        if events.is_empty() {
            return Err(Error::invalid(format!("Issue '{issue}' has no stored events.")));
        }

        events[0]
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Error::invalid(format!("Issue '{issue}' has no stored events.")))
    }
}

fn extract_cursor(next_url: Option<&str>) -> Option<String> {
    let next_url = next_url?.trim();
    if next_url.is_empty() {
        return None;
    }
    let query = if let Some((_, q)) = next_url.split_once('?') { q } else { next_url };
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == "cursor"
        {
            return percent_decode(v);
        }
    }
    None
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::new();
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next()?;
            let h2 = bytes.next()?;
            let hex = [h1, h2];
            let str_hex = std::str::from_utf8(&hex).ok()?;
            let val = u8::from_str_radix(str_hex, 16).ok()?;
            out.push(val);
        } else if b == b'+' {
            out.push(b' ');
        } else {
            out.push(b);
        }
    }
    String::from_utf8(out).ok()
}

fn matches_key(row: &Value, key: &str, value: &str) -> bool {
    row.get(key).and_then(Value::as_str).is_some_and(|s| s.eq_ignore_ascii_case(value))
}

fn read_json(mut response: Response<ureq::Body>) -> Result<Value> {
    let status = response.status().as_u16();
    let reason = response.status().canonical_reason().unwrap_or("Unknown");
    let bytes = response.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(transport_error)?;

    if !(200..300).contains(&status) {
        let body = String::from_utf8_lossy(&bytes).trim().to_string();
        return Err(status_error(status, reason, &body));
    }

    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(Value::Object(Default::default()));
    }

    serde_json::from_slice(&bytes).or_else(|_| Ok(Value::String(String::from_utf8_lossy(&bytes).into_owned())))
}

fn read_text(mut response: Response<ureq::Body>) -> Result<String> {
    let status = response.status().as_u16();
    let reason = response.status().canonical_reason().unwrap_or("Unknown");
    let bytes = response.body_mut().with_config().limit(MAX_BODY).read_to_vec().map_err(transport_error)?;

    if !(200..300).contains(&status) {
        let body = String::from_utf8_lossy(&bytes).trim().to_string();
        return Err(status_error(status, reason, &body));
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn transport_error(e: ureq::Error) -> Error {
    match e {
        ureq::Error::Timeout(_) => {
            Error::new(ErrorCode::Network, "The request timed out.").fix("Retry once, then stop.")
        }
        other => Error::new(ErrorCode::Network, "Could not reach the Bugsink API.")
            .detail(other.to_string())
            .fix("Retry once, then stop."),
    }
}

pub fn status_error(status: u16, reason: &str, body: &str) -> Error {
    let prefix = format!("{status} {reason}");
    let mut message = prefix.clone();
    let mut detail_msg = String::new();

    if let Ok(parsed) = serde_json::from_str::<Value>(body) {
        if let Some(detail) = parsed.get("detail").and_then(Value::as_str) {
            message = format!("{prefix}: {detail}");
        } else if let Some(map) = parsed.as_object() {
            let fields: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let desc = match v {
                        Value::Array(arr) => arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(" "),
                        Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    };
                    format!("{k}: {desc}")
                })
                .collect();
            if !fields.is_empty() {
                message = format!("{prefix}: {}", fields.join("; "));
            }
        }
    } else if !body.is_empty() {
        message = format!("{prefix}: {body}");
    }

    if !body.is_empty() {
        detail_msg = format!("HTTP {status}: {body}");
    }

    let mut err = match status {
        401 => Error::new(ErrorCode::AuthRequired, message)
            .fix("Check your API token (--api-token or BUGSINK_API_TOKEN), or: bugsink accounts add <name> --api-token <token> --force"),
        403 => Error::new(ErrorCode::AuthRequired, message)
            .fix("Check that your token has permission for this operation."),
        404 => Error::new(ErrorCode::NotFound, message),
        429 => Error::new(ErrorCode::RateLimited, message)
            .fix("Back off before retrying."),
        400 | 422 => Error::new(ErrorCode::InvalidInput, message),
        s if s >= 500 => Error::new(ErrorCode::Network, message)
            .fix("Retry once; if it persists the Bugsink server is having trouble."),
        _ => Error::new(ErrorCode::Error, message),
    };

    if !detail_msg.is_empty() {
        err = err.detail(detail_msg);
    }
    err
}

/// Percent-encodes one path segment or query value.
pub fn seg(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_escapes_reserved() {
        assert_eq!(seg("project-1"), "project-1");
        assert_eq!(seg("A B/C"), "A%20B%2FC");
    }

    #[test]
    fn cursor_extraction() {
        let next = "http://example.com/api/canonical/0/projects/?cursor=cD0yMDI2LTA5LTI0";
        assert_eq!(extract_cursor(Some(next)), Some("cD0yMDI2LTA5LTI0".into()));
        assert_eq!(extract_cursor(None), None);
    }

    #[test]
    fn status_error_formatting() {
        let err = status_error(401, "Unauthorized", r#"{"detail":"Malformed Bearer token"}"#);
        assert_eq!(err.code, ErrorCode::AuthRequired);
        assert!(err.message.contains("Malformed Bearer token"));

        let err = status_error(404, "Not Found", "");
        assert_eq!(err.code, ErrorCode::NotFound);
    }
}
