//! Issues command handlers.

use std::io::Read;

use serde_json::{Value, json};

use crate::cli::AccountArgs;
use crate::client::seg;
use crate::commands::{client, done, print};
use crate::error::{Error, Result};
use crate::{obj, output};

pub fn list(
    project: String,
    sort: Option<String>,
    order: Option<String>,
    status: String,
    limit: usize,
    all: bool,
    a: AccountArgs,
) -> Result<()> {
    let cl = client(&a)?;
    let project_id = cl.resolve_project_id(&project)?;

    let mut path = format!("issues/?project={project_id}");
    if let Some(s) = sort.filter(|s| !s.trim().is_empty()) {
        path.push_str(&format!("&sort={}", seg(&s)));
    }
    if let Some(o) = order.filter(|o| !o.trim().is_empty()) {
        path.push_str(&format!("&order={}", seg(&o)));
    }

    let status_filter = status.to_lowercase();
    type FilterFn = Box<dyn Fn(&Value) -> bool>;
    let keep: Option<FilterFn> = match status_filter.as_str() {
        "all" => None,
        "open" => Some(Box::new(|i| !flag(i, "is_resolved") && !flag(i, "is_muted"))),
        "unresolved" => Some(Box::new(|i| !flag(i, "is_resolved"))),
        "resolved" => Some(Box::new(|i| flag(i, "is_resolved"))),
        "muted" => Some(Box::new(|i| flag(i, "is_muted"))),
        _ => {
            return Err(Error::invalid(format!(
                "Unknown --status '{status}'. Use all, open, unresolved, resolved or muted."
            )));
        }
    };

    let effective_limit = if all { None } else { Some(limit) };
    let items = cl.get_paged(&path, effective_limit, keep.as_ref().map(|k| move |v: &Value| k(v)))?;
    print(Value::Array(items))
}

fn flag(issue: &Value, key: &str) -> bool {
    issue.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub fn get(issue: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("issues/{}/", seg(&issue));
    print(cl.get(&path)?)
}

pub fn stacktrace(issue: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let event_id = cl.resolve_latest_event_id(&issue)?;
    let path = format!("events/{}/stacktrace/", seg(&event_id));
    let text = cl.get_text(&path)?;
    if output::json() {
        print(obj! { "stacktrace" => text })
    } else {
        println!("{}", text.trim_end());
        Ok(())
    }
}

pub fn resolve(issue: String, latest_release: bool, next_release: bool, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let action = if latest_release {
        "resolve-latest/"
    } else if next_release {
        "resolve-next/"
    } else {
        "resolve/"
    };
    let path = format!("issues/{}/{action}", seg(&issue));
    let res = cl.post_empty(&path)?;
    done(res, "resolved", obj! { "issue" => &issue })
}

pub fn reopen(issue: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("issues/{}/reopen/", seg(&issue));
    let res = cl.post_empty(&path)?;
    done(res, "reopened", obj! { "issue" => &issue })
}

pub fn mute(
    issue: String,
    periods: Option<i32>,
    period: Option<String>,
    threshold: Option<i32>,
    a: AccountArgs,
) -> Result<()> {
    let cl = client(&a)?;
    let (action, body) = if let Some(thresh) = threshold {
        if periods.is_none() || period.is_none() {
            return Err(Error::invalid("--threshold also needs --for <N> and --period <PERIOD>."));
        }
        (
            "mute-until/",
            Some(json!({
                "period_name": period.unwrap(),
                "nr_of_periods": periods.unwrap(),
                "gte_threshold": thresh,
            })),
        )
    } else if periods.is_some() || period.is_some() {
        if periods.is_none() || period.is_none() {
            return Err(Error::invalid("--for and --period must be given together."));
        }
        (
            "mute-for/",
            Some(json!({
                "period_name": period.unwrap(),
                "nr_of_periods": periods.unwrap(),
            })),
        )
    } else {
        ("mute/", None)
    };

    let path = format!("issues/{}/{action}", seg(&issue));
    let res = match body {
        Some(b) => cl.post(&path, &b)?,
        None => cl.post_empty(&path)?,
    };
    done(res, "muted", obj! { "issue" => &issue })
}

pub fn unmute(issue: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("issues/{}/unmute/", seg(&issue));
    let res = cl.post_empty(&path)?;
    done(res, "unmuted", obj! { "issue" => &issue })
}

pub fn comment(issue: String, text: Option<String>, stdin: bool, a: AccountArgs) -> Result<()> {
    let text = if stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| Error::invalid(format!("Could not read comment from stdin: {e}")))?;
        buf
    } else if let Some(t) = text {
        t
    } else {
        return Err(Error::invalid("Pass exactly one of --text <TEXT> or --stdin."));
    };

    let comment_body = text.trim_end().to_string();
    if comment_body.trim().is_empty() {
        return Err(Error::invalid("Comment text is empty."));
    }

    let cl = client(&a)?;
    let body = json!({
        "issue": issue,
        "comment": comment_body,
    });
    let res = cl.post("issue-comments/", &body)?;
    done(res, "commented", obj! { "issue" => &issue })
}

pub fn delete(issue: String, a: AccountArgs) -> Result<()> {
    let cl = client(&a)?;
    let path = format!("issues/{}/", seg(&issue));
    let res = cl.delete(&path)?;
    done(res, "deleted", obj! { "issue" => &issue })
}
