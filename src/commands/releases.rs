//! Releases command handlers.

use serde_json::{Value, json};

use crate::cli::AccountArgs;
use crate::client::seg;
use crate::commands::{client, print};
use crate::error::Result;
use crate::obj;

pub fn list(project: String, limit: usize, all: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let project_id = cl.resolve_project_id(&project)?;
    let path = format!("releases/?project={project_id}");
    let effective_limit = if all { None } else { Some(limit) };
    let items = cl.get_paged(&path, effective_limit, None::<fn(&Value) -> bool>)?;
    print(Value::Array(items))
}

pub fn get(release: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("releases/{}/", seg(&release));
    print(cl.get(&path)?)
}

pub fn create(project: String, version: String, timestamp: Option<String>, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let project_id = cl.resolve_project_id(&project)?;

    let mut body = obj! {
        "project" => project_id,
        "version" => version,
    };
    if let Some(ts) = timestamp {
        body["timestamp"] = json!(ts);
    }

    print(cl.post("releases/", &body)?)
}
