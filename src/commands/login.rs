//! `bugsink login`. Authenticates with Bugsink via API token and base URL,
//! verifying against the API, and storing credentials in the OS keystore.

use std::io::{BufRead, IsTerminal, Write};

use crate::cli::LoginArgs;
use crate::client::Client;
use crate::commands::print;
use crate::config::{self, AccountConfig};
use crate::error::{Error, Result};
use crate::obj;
use crate::secrets;

pub fn run(args: LoginArgs) -> Result<()> {
    let LoginArgs { name, endpoint, api_token, api_token_stdin, no_browser, force, no_verify } = args;

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
        return Err(Error::invalid(format!("An account named '{existing}' already exists."))
            .fix(format!("Use --force to replace its credentials: bugsink login {existing} --force")));
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

    let token = if api_token_stdin {
        read_stdin_token()?
    } else if let Some(k) = api_token
        .or_else(|| std::env::var("BUGSINK_API_TOKEN").ok())
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
    {
        k
    } else {
        prompt_login_token(&name, &endpoint, no_browser)?
    };

    if !no_verify {
        // Verify before storing: probe projects/ endpoint with limit 1
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

    if std::io::stderr().is_terminal() {
        eprintln!("Successfully logged in to Bugsink at {endpoint} as account '{name}'.");
    }

    print(obj! {
        "status" => "logged_in",
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

fn prompt_login_token(name: &str, endpoint: &str, no_browser: bool) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::invalid("No API token given and no terminal to prompt on.")
            .fix(format!("pbpaste | bugsink login {name} --api-token-stdin")));
    }

    eprintln!("To log in, copy or create an API token from your Bugsink server:");
    eprintln!("  {endpoint}\n");

    if !no_browser {
        eprintln!("Opening {endpoint} in your browser...");
        open_browser(endpoint);
    }

    let _ = std::io::stderr().flush();

    loop {
        let key = rpassword::prompt_password(format!("Paste your Bugsink API token for '{name}': "))
            .map_err(|e| Error::other("Could not read the API token.").detail(e.to_string()))?;
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
        eprintln!("Token cannot be empty.");
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
