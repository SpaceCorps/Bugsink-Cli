//! `accounts add|list|test|remove`. Commands that manage what `--account` refers to.

use std::io::{BufRead, IsTerminal, Write};

use serde_json::Value;

use crate::account;
use crate::cli::AccountsCommand;
use crate::client::Client;
use crate::commands::print;
use crate::config::{self, AccountConfig};
use crate::error::{Error, ErrorCode, Result};
use crate::obj;
use crate::secrets::{self, Store};

pub fn run(c: AccountsCommand) -> Result<()> {
    match c {
        AccountsCommand::Add { name, endpoint, api_token, api_token_stdin, force, no_verify } => {
            let api_token = if api_token_stdin { Some(read_stdin_token()?) } else { api_token };
            add(name, endpoint, api_token, force, no_verify)
        }
        AccountsCommand::List { check } => list(check),
        AccountsCommand::Test { name } => test(&name),
        AccountsCommand::Remove { name, yes } => remove(&name, yes),
    }
}

fn add(name: String, endpoint: Option<String>, api_token: Option<String>, force: bool, no_verify: bool) -> Result<()> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::invalid("An account name is required."));
    }

    let store = secrets::store()?;
    let config = config::load()?;

    let existing = config.find(&name).map(|(k, _)| k.clone());
    if let Some(existing) = &existing
        && !force
    {
        return Err(Error::invalid(format!("An account named '{existing}' already exists.")).fix(format!(
            "Pick a different name, or replace its credentials: bugsink accounts add {existing} --endpoint <url> --api-token <token> --force"
        )));
    }
    let name = existing.clone().unwrap_or(name);

    let endpoint = match endpoint
        .or_else(|| std::env::var("BUGSINK_ENDPOINT").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        Some(ep) => ep,
        None => prompt_endpoint()?,
    };

    let token = match api_token
        .or_else(|| std::env::var("BUGSINK_API_TOKEN").ok())
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
    {
        Some(k) => k,
        None => prompt_token(&name)?,
    };

    if !no_verify {
        let client = Client::new(&endpoint, &token);
        let _ = client.get("projects/?limit=1")?;
    }

    let identity = host_label(&endpoint);

    {
        let _lock = config::lock()?;
        store.set(&secrets::account_key(&name), &token)?;

        let mut config = config::load()?;
        config.accounts.insert(
            name.clone(),
            AccountConfig { endpoint: endpoint.clone(), identity: identity.clone(), added_at: config::now_utc() },
        );
        config::save(&config)?;
    }

    print(obj! {
        "status" => if existing.is_none() { "added" } else { "replaced" },
        "name" => name,
        "endpoint" => endpoint,
        "identity" => identity,
        "verified" => !no_verify,
        "secretStore" => store.name(),
        "configDir" => config::config_dir().display().to_string(),
        "nextStep" => format!("bugsink projects list --account {name}"),
    })
}

fn host_label(endpoint: &str) -> String {
    let clean = endpoint.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
    clean.split('/').next().unwrap_or(clean).to_string()
}

fn read_stdin_token() -> Result<String> {
    let mut key = String::new();
    std::io::stdin().lock().read_line(&mut key).map_err(|e| Error::invalid(format!("Could not read stdin: {e}")))?;
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(Error::invalid("--api-token-stdin was given but stdin was empty."));
    }
    Ok(key)
}

fn prompt_endpoint() -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::invalid("No endpoint URL given and no terminal to prompt on.")
            .fix("Pass --endpoint <URL> or set BUGSINK_ENDPOINT"));
    }
    loop {
        eprint!("Enter Bugsink base URL (e.g. https://bugsink.example.com): ");
        let _ = std::io::stderr().flush();
        let mut ep = String::new();
        let _ = std::io::stdin().lock().read_line(&mut ep);
        let ep = ep.trim().to_string();
        if !ep.is_empty() {
            return Ok(ep);
        }
        eprintln!("Endpoint cannot be empty.");
    }
}

fn prompt_token(name: &str) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::invalid("No API token given and no terminal to prompt on.")
            .fix(format!("pbpaste | bugsink accounts add {name} --api-token-stdin")));
    }
    loop {
        let key = rpassword::prompt_password(format!("API token for {name}: "))
            .map_err(|e| Error::other("Could not read the API token.").detail(e.to_string()))?;
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
        eprintln!("Cannot be empty");
    }
}

fn list(check: bool) -> Result<()> {
    let store = secrets::store()?;
    let config = config::load()?;
    let sorted = config.sorted();

    let statuses: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> =
            sorted.iter().map(|(name, acct)| scope.spawn(move || status_of(name, acct, store, check))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| "unreachable".into())).collect()
    });

    let accounts: Vec<Value> = sorted
        .iter()
        .zip(statuses)
        .map(|((name, a), status)| {
            obj! {
                "name" => name,
                "endpoint" => a.endpoint,
                "identity" => a.identity,
                "addedAt" => a.added_at,
                "keyStatus" => status,
            }
        })
        .collect();

    print(obj! {
        "count" => accounts.len(),
        "accounts" => accounts,
        "secretStore" => store.name(),
        "configDir" => config::config_dir().display().to_string(),
    })
}

fn status_of(name: &str, acct: &AccountConfig, store: Store, check: bool) -> String {
    let key = match store.get(&secrets::account_key(name)) {
        Ok(Some(k)) if !k.trim().is_empty() => k,
        Ok(_) => return "missing_key".into(),
        Err(_) => return "unreadable".into(),
    };

    if !check {
        return "stored".into();
    }

    match Client::new(&acct.endpoint, &key).get("projects/?limit=1") {
        Ok(_) => "valid".into(),
        Err(e) if e.code == ErrorCode::AuthRequired => "rejected".into(),
        Err(_) => "unreachable".into(),
    }
}

fn test(name: &str) -> Result<()> {
    let account = account::resolve(Some(name), None, None)?;
    let client = account.client();
    let res = client.get("projects/?limit=1")?;

    print(obj! {
        "name" => account.name,
        "endpoint" => account.endpoint,
        "keyStatus" => "valid",
        "sample" => res,
    })
}

fn remove(requested: &str, yes: bool) -> Result<()> {
    let store = secrets::store()?;
    let config = config::load()?;

    let Some((name, acct)) = config.find(requested) else {
        return Err(
            Error::new(ErrorCode::NoAccount, format!("No account named '{requested}'.")).fix("bugsink accounts list")
        );
    };
    let (name, acct) = (name.clone(), acct.clone());

    if !yes {
        if !std::io::stdin().is_terminal() {
            return Err(Error::invalid(format!(
                "Removing '{name}' needs confirmation and there is no terminal to ask on."
            ))
            .fix(format!("bugsink accounts remove {name} --yes")));
        }
        eprint!("Remove account {name} ({})? [y/N] ", acct.endpoint);
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        let _ = std::io::stdin().lock().read_line(&mut answer);
        if !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            return Err(Error::invalid("Cancelled."));
        }
    }

    {
        let _lock = config::lock()?;
        store.delete(&secrets::account_key(&name))?;
        let mut config = config::load()?;
        config.accounts.shift_remove(&name);
        config::save(&config)?;
    }

    print(obj! {
        "status" => "removed",
        "name" => name,
        "endpoint" => acct.endpoint,
        "note" => "The API token was deleted locally.",
    })
}
