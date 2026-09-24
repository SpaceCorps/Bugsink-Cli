//! Account and credentials resolution.
//!
//! Accounts store an endpoint URL and non-secret metadata in config, and the API token in
//! the OS keystore. `--account <name>` (short `-a`) selects the account. Alternatively,
//! `--endpoint` and `--api-token` (or `BUGSINK_ENDPOINT` and `BUGSINK_API_TOKEN`) allow
//! explicit or environment-driven invocations.

use crate::client::Client;
use crate::config::{self, Config};
use crate::error::{Error, ErrorCode, Result};
use crate::secrets;

pub struct Resolved {
    pub name: String,
    pub endpoint: String,
    pub api_token: String,
}

impl Resolved {
    pub fn client(&self) -> Client {
        Client::new(&self.endpoint, &self.api_token)
    }
}

pub fn resolve(
    requested: Option<&str>,
    explicit_endpoint: Option<&str>,
    explicit_token: Option<&str>,
) -> Result<Resolved> {
    // 1. Direct flags or environment variables take precedence if no account name is specified
    let env_endpoint = std::env::var("BUGSINK_ENDPOINT").ok().filter(|s| !s.trim().is_empty());
    let env_token = std::env::var("BUGSINK_API_TOKEN").ok().filter(|s| !s.trim().is_empty());

    let opt_endpoint = explicit_endpoint.map(str::trim).filter(|s| !s.is_empty()).or(env_endpoint.as_deref());
    let opt_token = explicit_token.map(str::trim).filter(|s| !s.is_empty()).or(env_token.as_deref());

    let requested = requested.map(str::trim).filter(|s| !s.is_empty());

    if requested.is_none()
        && let (Some(ep), Some(tok)) = (opt_endpoint, opt_token)
    {
        return Ok(Resolved { name: "direct".into(), endpoint: ep.to_string(), api_token: tok.to_string() });
    }

    let config = config::load()?;

    let Some(requested) = requested else {
        if opt_endpoint.is_none() && opt_token.is_some() {
            return Err(Error::invalid("Bugsink URL required. Use --endpoint or set BUGSINK_ENDPOINT."));
        }
        if opt_endpoint.is_some() && opt_token.is_none() {
            return Err(Error::invalid("API token required. Use --api-token or set BUGSINK_API_TOKEN."));
        }

        return Err(Error::new(ErrorCode::NoAccount, "No account specified. Pass --account <name>.")
            .detail(describe(&config))
            .fix("bugsink accounts list"));
    };

    let Some((name, account)) = config.find(requested) else {
        return Err(Error::new(ErrorCode::NoAccount, format!("No account named '{requested}'."))
            .detail(describe(&config))
            .fix("bugsink accounts list"));
    };

    let key = secrets::store()?.get(&secrets::account_key(name))?;

    let Some(api_token) = key.filter(|k| !k.trim().is_empty()) else {
        return Err(Error::new(ErrorCode::AuthRequired, format!("Account '{name}' has no stored API token."))
            .detail("The config entry exists but the keystore has nothing under it.")
            .fix(format!("bugsink accounts add {name} --api-token <token>")));
    };

    Ok(Resolved { name: name.clone(), endpoint: account.endpoint.clone(), api_token })
}

fn describe(config: &Config) -> String {
    if config.accounts.is_empty() {
        return "No accounts are configured yet. Run 'bugsink accounts add <name> --endpoint <url> --api-token <token>'.".into();
    }
    let listed: Vec<String> = config
        .sorted()
        .into_iter()
        .map(|(k, v)| {
            if v.identity.trim().is_empty() {
                format!("{k} ({})", v.endpoint)
            } else {
                format!("{k} ({}, {})", v.identity, v.endpoint)
            }
        })
        .collect();
    format!("Configured accounts: {}", listed.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_env_resolves_when_no_account() {
        let res = resolve(None, Some("http://localhost:8000"), Some("token123")).unwrap();
        assert_eq!(res.endpoint, "http://localhost:8000");
        assert_eq!(res.api_token, "token123");
    }
}
