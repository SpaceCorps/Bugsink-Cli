//! The command tree for the Bugsink CLI.

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bugsink",
    version,
    about = "CLI for Bugsink - self-hosted error tracking with native speed and agentic control",
    after_help = "An LLM agent should start with: bugsink agent-readme",
    propagate_version = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Print raw JSON instead of YAML, for scripting
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone, Default)]
pub struct AccountArgs {
    /// Account to run against (see 'bugsink accounts list')
    #[arg(short = 'a', long, value_name = "ACCOUNT")]
    pub account: Option<String>,

    /// Bugsink base URL, e.g. https://bugsink.example.com (or set BUGSINK_ENDPOINT)
    #[arg(long, value_name = "URL")]
    pub endpoint: Option<String>,

    /// Bugsink API token (or set BUGSINK_API_TOKEN)
    #[arg(long, value_name = "TOKEN")]
    pub api_token: Option<String>,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Print the operating manual for an LLM agent driving this CLI
    AgentReadme,
    /// Log in with a Bugsink API token and endpoint
    Login(LoginArgs),
    /// Manage Bugsink accounts and stored credentials
    #[command(subcommand)]
    Accounts(AccountsCommand),
    /// Triage issues
    #[command(subcommand)]
    Issues(IssuesCommand),
    /// Read the events behind an issue
    #[command(subcommand)]
    Events(EventsCommand),
    /// Manage projects
    #[command(subcommand)]
    Projects(ProjectsCommand),
    /// Manage teams
    #[command(subcommand)]
    Teams(TeamsCommand),
    /// Manage releases
    #[command(subcommand)]
    Releases(ReleasesCommand),
}

// ---------------------------------------------------------------------------------------------
// login & accounts

#[derive(Args, Clone)]
pub struct LoginArgs {
    /// Account name to store (default: "default")
    #[arg(value_name = "NAME", default_value = "default")]
    pub name: String,

    /// Bugsink base URL (prompted if omitted, or set BUGSINK_ENDPOINT)
    #[arg(long, value_name = "URL")]
    pub endpoint: Option<String>,

    /// Bugsink API token (prompted without echo if omitted)
    #[arg(long, value_name = "TOKEN", conflicts_with = "api_token_stdin")]
    pub api_token: Option<String>,

    /// Read the API token from stdin
    #[arg(long)]
    pub api_token_stdin: bool,

    /// Do not open the browser to the Bugsink UI automatically
    #[arg(long)]
    pub no_browser: bool,

    /// Replace the token on an account that already exists
    #[arg(long)]
    pub force: bool,

    /// Store credentials without calling the API to verify them first
    #[arg(long)]
    pub no_verify: bool,
}

#[derive(Subcommand)]
pub enum AccountsCommand {
    /// Add an account and store its API token in the OS keystore
    Add {
        /// Short name for this account, used as --account elsewhere
        name: String,

        /// Bugsink base URL, e.g. https://bugsink.example.com
        #[arg(long, value_name = "URL")]
        endpoint: Option<String>,

        /// Bugsink API token (prompted without echo if omitted)
        #[arg(long, value_name = "TOKEN", conflicts_with = "api_token_stdin")]
        api_token: Option<String>,

        /// Read the API token from stdin
        #[arg(long)]
        api_token_stdin: bool,

        /// Replace the credentials on an account that already exists
        #[arg(long)]
        force: bool,

        /// Store credentials without calling the API to verify them first
        #[arg(long)]
        no_verify: bool,
    },
    /// List configured accounts
    List {
        /// Call the API once per account to check token validity
        #[arg(long)]
        check: bool,
    },
    /// Check that an account's stored credentials still work
    Test {
        /// Account name
        name: String,
    },
    /// Remove an account and delete its stored token
    Remove {
        /// Account name
        name: String,

        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
}

// ---------------------------------------------------------------------------------------------
// issues

#[derive(Subcommand)]
pub enum IssuesCommand {
    /// List the issues of a project
    List {
        /// Project id, slug or name
        #[arg(long, value_name = "PROJECT")]
        project: String,

        /// digest_order, digested_event_count or last_seen; defaults to digest_order
        #[arg(long, value_name = "SORT")]
        sort: Option<String>,

        /// asc or desc; defaults to asc
        #[arg(long, value_name = "ORDER")]
        order: Option<String>,

        /// Filter by state: all, open, unresolved, resolved or muted; defaults to all
        #[arg(long, value_name = "STATUS", default_value = "all")]
        status: String,

        /// Maximum number of items to return; default 50
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Return every item, following pagination to the end
        #[arg(long)]
        all: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Get one issue by UUID or friendly id
    Get {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Print the stacktrace of the issue's most recent event
    Stacktrace {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Mark an issue resolved, optionally in the latest or next release
    Resolve {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        /// Resolve in the latest release: reopens if it recurs in a newer one
        #[arg(long, conflicts_with = "next_release")]
        latest_release: bool,

        /// Resolve in the next release: reopens if it recurs after the next release
        #[arg(long, conflicts_with = "latest_release")]
        next_release: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Mark a resolved issue unresolved again
    Reopen {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Mute an issue, for a period or until it gets noisy again
    Mute {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        /// Number of periods, used with --period
        #[arg(long = "for", value_name = "N")]
        periods: Option<i32>,

        /// year, month, week, day, hour or minute
        #[arg(long, value_name = "PERIOD")]
        period: Option<String>,

        /// Unmute once this many events arrive within the period
        #[arg(long, value_name = "N")]
        threshold: Option<i32>,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Unmute an issue
    Unmute {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Add a comment to an issue
    Comment {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        /// Comment body
        #[arg(long, value_name = "TEXT", conflicts_with = "stdin")]
        text: Option<String>,

        /// Read the comment body from stdin instead of --text
        #[arg(long, conflicts_with = "text")]
        stdin: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Delete an issue and its events
    Delete {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        issue: String,

        #[command(flatten)]
        a: AccountArgs,
    },
}

// ---------------------------------------------------------------------------------------------
// events

#[derive(Subcommand)]
pub enum EventsCommand {
    /// List the stored events of an issue
    List {
        /// Issue UUID or friendly id, e.g. MYPROJECT-7
        #[arg(long, value_name = "ISSUE")]
        issue: String,

        /// asc or desc; defaults to desc (newest first)
        #[arg(long, value_name = "ORDER", default_value = "desc")]
        order: String,

        /// Maximum number of items to return; default 50
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Return every item, following pagination to the end
        #[arg(long)]
        all: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Get one event, including its full payload
    Get {
        /// Bugsink-internal event UUID, as listed by 'events list'
        event: String,

        /// Omit the full event payload, which is large and mostly SDK metadata
        #[arg(long)]
        no_data: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Render one event's stacktrace as Markdown
    Stacktrace {
        /// Bugsink-internal event UUID, as listed by 'events list'
        event: String,

        #[command(flatten)]
        a: AccountArgs,
    },
}

// ---------------------------------------------------------------------------------------------
// projects

#[derive(Subcommand)]
pub enum ProjectsCommand {
    /// List projects, with their DSN and event counts
    List {
        /// Only list projects of this team (UUID or name)
        #[arg(long, value_name = "TEAM")]
        team: Option<String>,

        /// Maximum number of items to return; default 50
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Return every item, following pagination to the end
        #[arg(long)]
        all: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Get one project by id, slug or name
    Get {
        /// Project id, slug or name
        project: String,

        /// Include the full team object instead of just its UUID
        #[arg(long)]
        expand_team: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Create a project in a team
    Create {
        /// Owning team, by UUID or name
        #[arg(long, value_name = "TEAM")]
        team: String,

        /// Project name
        #[arg(long, value_name = "NAME")]
        name: String,

        /// joinable, discoverable or team_members
        #[arg(long, value_name = "VISIBILITY")]
        visibility: Option<String>,

        /// Send an alert the first time an issue is seen
        #[arg(long, value_name = "BOOL")]
        alert_on_new_issue: Option<bool>,

        /// Send an alert when a resolved issue comes back
        #[arg(long, value_name = "BOOL")]
        alert_on_regression: Option<bool>,

        /// Send an alert when a muted issue unmutes itself
        #[arg(long, value_name = "BOOL")]
        alert_on_unmute: Option<bool>,

        /// Maximum number of events to keep for this project
        #[arg(long, value_name = "N")]
        retention: Option<i64>,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Update a project's name, team, visibility, alerts or retention
    Update {
        /// Project id, slug or name
        project: String,

        /// Move the project to this team (UUID or name)
        #[arg(long, value_name = "TEAM")]
        team: Option<String>,

        /// New project name
        #[arg(long, value_name = "NAME")]
        name: Option<String>,

        /// joinable, discoverable or team_members
        #[arg(long, value_name = "VISIBILITY")]
        visibility: Option<String>,

        /// Send an alert the first time an issue is seen
        #[arg(long, value_name = "BOOL")]
        alert_on_new_issue: Option<bool>,

        /// Send an alert when a resolved issue comes back
        #[arg(long, value_name = "BOOL")]
        alert_on_regression: Option<bool>,

        /// Send an alert when a muted issue unmutes itself
        #[arg(long, value_name = "BOOL")]
        alert_on_unmute: Option<bool>,

        /// Maximum number of events to keep for this project
        #[arg(long, value_name = "N")]
        retention: Option<i64>,

        #[command(flatten)]
        a: AccountArgs,
    },
}

// ---------------------------------------------------------------------------------------------
// teams

#[derive(Subcommand)]
pub enum TeamsCommand {
    /// List all teams
    List {
        /// Maximum number of items to return; default 50
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Return every item, following pagination to the end
        #[arg(long)]
        all: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Get one team by UUID or name
    Get {
        /// Team UUID or name
        team: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Create a team
    Create {
        /// Team name
        #[arg(long, value_name = "NAME")]
        name: String,

        /// joinable, discoverable or hidden; defaults to discoverable
        #[arg(long, value_name = "VISIBILITY")]
        visibility: Option<String>,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Update a team's name or visibility
    Update {
        /// Team UUID or name
        team: String,

        /// New team name
        #[arg(long, value_name = "NAME")]
        name: Option<String>,

        /// joinable, discoverable or hidden
        #[arg(long, value_name = "VISIBILITY")]
        visibility: Option<String>,

        #[command(flatten)]
        a: AccountArgs,
    },
}

// ---------------------------------------------------------------------------------------------
// releases

#[derive(Subcommand)]
#[command(disable_version_flag = true)]
pub enum ReleasesCommand {
    /// List the releases of a project
    List {
        /// Project id, slug or name
        #[arg(long, value_name = "PROJECT")]
        project: String,

        /// Maximum number of items to return; default 50
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Return every item, following pagination to the end
        #[arg(long)]
        all: bool,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Get one release by UUID
    Get {
        /// Release UUID, as listed by 'releases list'
        release: String,

        #[command(flatten)]
        a: AccountArgs,
    },
    /// Create a release for a project
    Create {
        /// Project id, slug or name
        #[arg(long, value_name = "PROJECT")]
        project: String,

        /// Release version string, e.g. my-package@1.2.3
        #[arg(long, value_name = "VERSION")]
        version: String,

        /// Release date as ISO 8601; defaults to now on the server
        #[arg(long, value_name = "TIMESTAMP")]
        timestamp: Option<String>,

        #[command(flatten)]
        a: AccountArgs,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    #[test]
    fn command_tree_is_valid() {
        super::Cli::command().debug_assert();
    }
}
