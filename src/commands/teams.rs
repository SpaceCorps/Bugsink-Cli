//! Teams command handlers.

use serde_json::{Value, json};

use crate::cli::AccountArgs;
use crate::commands::{client, print};
use crate::error::{Error, Result};
use crate::obj;

pub fn list(limit: usize, all: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let effective_limit = if all { None } else { Some(limit) };
    let items = cl.get_paged("teams/", effective_limit, None::<fn(&Value) -> bool>)?;
    print(Value::Array(items))
}

pub fn get(team: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let team_id = cl.resolve_team_id(&team)?;
    let path = format!("teams/{team_id}/");
    print(cl.get(&path)?)
}

pub fn create(name: String, visibility: Option<String>, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let mut body = obj! { "name" => name };
    if let Some(v) = visibility {
        body["visibility"] = json!(v);
    }
    print(cl.post("teams/", &body)?)
}

pub fn update(team: String, name: Option<String>, visibility: Option<String>, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;

    let mut body = obj! {};
    if let Some(n) = name {
        body["name"] = json!(n);
    }
    if let Some(v) = visibility {
        body["visibility"] = json!(v);
    }

    if body.as_object().is_some_and(|m| m.is_empty()) {
        return Err(Error::invalid("Nothing to update. Pass --name and/or --visibility."));
    }

    let team_id = cl.resolve_team_id(&team)?;
    let path = format!("teams/{team_id}/");
    print(cl.patch(&path, &body)?)
}
