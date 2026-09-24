//! Projects command handlers.

use serde_json::{Value, json};

use crate::cli::AccountArgs;
use crate::client::seg;
use crate::commands::{client, print};
use crate::error::{Error, Result};
use crate::obj;

pub fn list(team: Option<String>, limit: usize, all: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let mut path = "projects/".to_string();
    if let Some(t) = team.filter(|t| !t.trim().is_empty()) {
        let team_id = cl.resolve_team_id(&t)?;
        path = format!("projects/?team={}", seg(&team_id));
    }
    let effective_limit = if all { None } else { Some(limit) };
    let items = cl.get_paged(&path, effective_limit, None::<fn(&Value) -> bool>)?;
    print(Value::Array(items))
}

pub fn get(project: String, expand_team: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let project_id = cl.resolve_project_id(&project)?;
    let mut path = format!("projects/{project_id}/");
    if expand_team {
        path.push_str("?expand=team");
    }
    print(cl.get(&path)?)
}

#[allow(clippy::too_many_arguments)]
pub fn create(
    team: String,
    name: String,
    visibility: Option<String>,
    alert_on_new_issue: Option<bool>,
    alert_on_regression: Option<bool>,
    alert_on_unmute: Option<bool>,
    retention: Option<i64>,
    a: AccountArgs,
) -> Result<()> {
    let cl = client(&a)?;
    let team_id = cl.resolve_team_id(&team)?;

    let mut body = obj! {
        "team" => team_id,
        "name" => name,
    };
    if let Some(v) = visibility {
        body["visibility"] = json!(v);
    }
    if let Some(a) = alert_on_new_issue {
        body["alert_on_new_issue"] = json!(a);
    }
    if let Some(a) = alert_on_regression {
        body["alert_on_regression"] = json!(a);
    }
    if let Some(a) = alert_on_unmute {
        body["alert_on_unmute"] = json!(a);
    }
    if let Some(r) = retention {
        body["retention_max_event_count"] = json!(r);
    }

    print(cl.post("projects/", &body)?)
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    project: String,
    team: Option<String>,
    name: Option<String>,
    visibility: Option<String>,
    alert_on_new_issue: Option<bool>,
    alert_on_regression: Option<bool>,
    alert_on_unmute: Option<bool>,
    retention: Option<i64>,
    a: AccountArgs,
) -> Result<()> {
    let cl = client(&a)?;

    let mut body = obj! {};
    if let Some(t) = team {
        body["team"] = json!(cl.resolve_team_id(&t)?);
    }
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(v) = visibility {
        body["visibility"] = json!(v);
    }
    if let Some(a) = alert_on_new_issue {
        body["alert_on_new_issue"] = json!(a);
    }
    if let Some(a) = alert_on_regression {
        body["alert_on_regression"] = json!(a);
    }
    if let Some(a) = alert_on_unmute {
        body["alert_on_unmute"] = json!(a);
    }
    if let Some(r) = retention {
        body["retention_max_event_count"] = json!(r);
    }

    if body.as_object().is_some_and(|m| m.is_empty()) {
        return Err(Error::invalid("Nothing to update. Pass at least one field option."));
    }

    let project_id = cl.resolve_project_id(&project)?;
    let path = format!("projects/{project_id}/");
    print(cl.patch(&path, &body)?)
}
