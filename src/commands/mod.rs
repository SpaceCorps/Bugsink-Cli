//! Subcommand dispatch and shared helpers.

mod accounts;
mod events;
mod issues;
mod login;
mod projects;
mod releases;
mod teams;

use serde_json::Value;

use crate::account::{self, Resolved};
use crate::cli::*;
use crate::client::Client;
use crate::error::Result;
use crate::{obj, output};

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::AgentReadme => {
            crate::readme::print();
            Ok(())
        }
        Command::Login(c) => login::run(c),
        Command::Accounts(c) => accounts::run(c),
        Command::Issues(c) => issues(c),
        Command::Events(c) => events(c),
        Command::Projects(c) => projects(c),
        Command::Teams(c) => teams(c),
        Command::Releases(c) => releases(c),
    }
}

pub(crate) fn resolve(a: &AccountArgs) -> Result<Resolved> {
    account::resolve(a.account.as_deref(), a.endpoint.as_deref(), a.api_token.as_deref())
}

pub(crate) fn client(a: &AccountArgs) -> Result<Client> {
    Ok(resolve(a)?.client())
}

pub(crate) fn print(v: Value) -> Result<()> {
    output::write(&v);
    Ok(())
}

/// For endpoints that answer 200/204 with no body: say what happened instead of printing `{}`,
/// and keep stdout parseable under --json.
pub(crate) fn done(mut v: Value, status: &str, fields: Value) -> Result<()> {
    match &mut v {
        Value::Object(o) => {
            o.insert("status".into(), Value::String(status.into()));
            if let Value::Object(f) = fields {
                for (k, val) in f {
                    o.entry(k).or_insert(val);
                }
            }
            print(v)
        }
        Value::Null => {
            let mut out = obj! { "status" => status };
            if let (Value::Object(o), Value::Object(f)) = (&mut out, fields) {
                o.extend(f);
            }
            print(out)
        }
        _ => print(v),
    }
}

fn issues(c: IssuesCommand) -> Result<()> {
    match c {
        IssuesCommand::List { project, sort, order, status, limit, all, a } => {
            issues::list(project, sort, order, status, limit, all, a)
        }
        IssuesCommand::Get { issue, a } => issues::get(issue, a),
        IssuesCommand::Stacktrace { issue, a } => issues::stacktrace(issue, a),
        IssuesCommand::Resolve { issue, latest_release, next_release, a } => {
            issues::resolve(issue, latest_release, next_release, a)
        }
        IssuesCommand::Reopen { issue, a } => issues::reopen(issue, a),
        IssuesCommand::Mute { issue, periods, period, threshold, a } => {
            issues::mute(issue, periods, period, threshold, a)
        }
        IssuesCommand::Unmute { issue, a } => issues::unmute(issue, a),
        IssuesCommand::Comment { issue, text, stdin, a } => issues::comment(issue, text, stdin, a),
        IssuesCommand::Delete { issue, a } => issues::delete(issue, a),
    }
}

fn events(c: EventsCommand) -> Result<()> {
    match c {
        EventsCommand::List { issue, order, limit, all, a } => events::list(issue, order, limit, all, a),
        EventsCommand::Get { event, no_data, a } => events::get(event, no_data, a),
        EventsCommand::Stacktrace { event, a } => events::stacktrace(event, a),
    }
}

fn projects(c: ProjectsCommand) -> Result<()> {
    match c {
        ProjectsCommand::List { team, limit, all, a } => projects::list(team, limit, all, a),
        ProjectsCommand::Get { project, expand_team, a } => projects::get(project, expand_team, a),
        ProjectsCommand::Create {
            team,
            name,
            visibility,
            alert_on_new_issue,
            alert_on_regression,
            alert_on_unmute,
            retention,
            a,
        } => projects::create(
            team,
            name,
            visibility,
            alert_on_new_issue,
            alert_on_regression,
            alert_on_unmute,
            retention,
            a,
        ),
        ProjectsCommand::Update {
            project,
            team,
            name,
            visibility,
            alert_on_new_issue,
            alert_on_regression,
            alert_on_unmute,
            retention,
            a,
        } => projects::update(
            project,
            team,
            name,
            visibility,
            alert_on_new_issue,
            alert_on_regression,
            alert_on_unmute,
            retention,
            a,
        ),
    }
}

fn teams(c: TeamsCommand) -> Result<()> {
    match c {
        TeamsCommand::List { limit, all, a } => teams::list(limit, all, a),
        TeamsCommand::Get { team, a } => teams::get(team, a),
        TeamsCommand::Create { name, visibility, a } => teams::create(name, visibility, a),
        TeamsCommand::Update { team, name, visibility, a } => teams::update(team, name, visibility, a),
    }
}

fn releases(c: ReleasesCommand) -> Result<()> {
    match c {
        ReleasesCommand::List { project, limit, all, a } => releases::list(project, limit, all, a),
        ReleasesCommand::Get { release, a } => releases::get(release, a),
        ReleasesCommand::Create { project, version, timestamp, a } => releases::create(project, version, timestamp, a),
    }
}
