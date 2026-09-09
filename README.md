# Bugsink.Console

A .NET CLI for [Bugsink](https://www.bugsink.com/), the self-hosted error tracker. Triage issues,
read stacktraces, and manage teams, projects and releases from a terminal or a script.

Output is YAML on stdout; status and error lines go to stderr, so `bugsink issues list ... | yq`
works on every command.

## Install

```bash
dotnet tool install --global Bugsink.Console
```

## Configure

```bash
export BUGSINK_ENDPOINT=https://bugsink.example.com
export BUGSINK_API_TOKEN=<40 lowercase hex chars>
```

Both can be overridden per command with `--endpoint` and `--api-token`. Create a token in the
Bugsink UI under your user settings.

## Issues

Issues are addressed by UUID or by the friendly id shown in the UI and in alert emails
(`MYPROJECT-7`). Projects are addressed by numeric id, slug or name.

```bash
# What is broken right now
bugsink issues list --project my-project --status open --sort last_seen --order desc --limit 20

# Why it is broken: the stacktrace of the issue's most recent event
bugsink issues stacktrace MYPROJECT-7

bugsink issues get MYPROJECT-7
bugsink issues comment MYPROJECT-7 --text "Fixed in #431."
git log -1 --format=%B | bugsink issues comment MYPROJECT-7 --stdin

bugsink issues resolve MYPROJECT-7                    # resolved for good
bugsink issues resolve MYPROJECT-7 --latest-release   # reopens if it recurs in a newer release
bugsink issues resolve MYPROJECT-7 --next-release     # reopens if it recurs after the next release
bugsink issues reopen MYPROJECT-7

bugsink issues mute MYPROJECT-7                                   # until unmuted
bugsink issues mute MYPROJECT-7 --for 3 --period day              # for three days
bugsink issues mute MYPROJECT-7 --threshold 10 --for 1 --period hour   # until 10 events in an hour

bugsink issues unmute MYPROJECT-7
bugsink issues delete MYPROJECT-7
```

`--status` filters client-side: `all` (default), `open` (neither resolved nor muted), `unresolved`,
`resolved`, `muted`.

## Events

```bash
bugsink events list --issue MYPROJECT-7 --limit 10
bugsink events get <event-uuid>              # includes the full payload
bugsink events get <event-uuid> --no-data    # metadata and rendered stacktrace only
bugsink events stacktrace <event-uuid>
```

## Projects, teams and releases

```bash
bugsink projects list [--team "My Team"]
bugsink projects get my-project [--expand-team]
bugsink projects create --team "My Team" --name "My Project" --visibility team_members
bugsink projects update my-project --retention 50000 --alert-on-regression true

bugsink teams list
bugsink teams get "My Team"
bugsink teams create --name "My Team" --visibility discoverable
bugsink teams update "My Team" --name "Renamed Team"

bugsink releases list --project my-project
bugsink releases get <release-uuid>
bugsink releases create --project my-project --version my-package@1.2.3
```

## Pagination

Every list command takes `--limit <N>` (default 50) or `--all`, and follows Bugsink's cursor
pagination for you.

## Scope

This wraps the Bugsink-specific `/api/canonical/0/` API. The Sentry-compatible ingest and
sourcemap-upload endpoints are out of scope — use `sentry-cli` or a Sentry SDK for those.
