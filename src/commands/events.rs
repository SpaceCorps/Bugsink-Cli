//! Events command handlers.

use serde_json::Value;

use crate::cli::AccountArgs;
use crate::client::seg;
use crate::commands::{client, print};
use crate::error::Result;
use crate::{obj, output};

pub fn list(issue: String, order: String, limit: usize, all: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("events/?issue={}&order={}", seg(&issue), seg(&order));
    let effective_limit = if all { None } else { Some(limit) };
    let items = cl.get_paged(&path, effective_limit, None::<fn(&Value) -> bool>)?;
    print(Value::Array(items))
}

pub fn get(event: String, no_data: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("events/{}/", seg(&event));
    let mut res = cl.get(&path)?;
    if no_data && let Some(map) = res.as_object_mut() {
        map.remove("data");
    }
    print(res)
}

pub fn stacktrace(event: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("events/{}/stacktrace/", seg(&event));
    let text = cl.get_text(&path)?;
    if output::json() {
        print(obj! { "stacktrace" => text })
    } else {
        println!("{}", text.trim_end());
        Ok(())
    }
}
