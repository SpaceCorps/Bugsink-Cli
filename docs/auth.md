---
title: "Authentication Guide"
description: "Authentication methods, credential storage, and error handling for developers and AI agents using the Bugsink CLI."
author: "SpaceCorps"
date: "2026-09-24"
---

# Authentication Guide for Bugsink CLI

This document outlines authentication methods, credential storage, and error handling for developers and AI agents using the Bugsink CLI.

## Overview
The Bugsink CLI interfaces directly with your Bugsink server's canonical REST API (`api/canonical/0/`). Authentication uses Bearer tokens (40 lowercase hex characters) issued by your Bugsink instance. Tokens can be stored in the host operating system's native keystore or supplied directly via environment variables and standard input.

## Prerequisites
- A self-hosted or managed Bugsink instance (e.g. `https://bugsink.example.com` or `http://localhost:8000`)
- A valid Bugsink API token
- Bugsink CLI installed on your machine (`cargo install --git https://github.com/SpaceCorps/Bugsink-Cli --locked`)

## Authentication Flow

### Interactive Browser Login (`bugsink login`)
The recommended flow for local developer machines:
```bash
bugsink login [account_name] --endpoint https://bugsink.example.com
```
1. The CLI launches your system browser to the server endpoint.
2. You generate or copy your personal API token.
3. Paste the token into the CLI prompt (input characters are masked).
4. The CLI validates the key with a live request to `GET projects/?limit=1`.
5. Upon confirmation, the key is securely saved to the native OS keyring under the account name (defaults to `default`).

### Non-Interactive / Headless Login
For headless CI/CD environments, Docker containers, or autonomous agent runners:
```bash
echo "$BUGSINK_API_TOKEN" | bugsink login [account_name] --endpoint "$BUGSINK_ENDPOINT" --api-token-stdin
```
Or pass the token directly as a CLI flag:
```bash
bugsink accounts add work --endpoint "$BUGSINK_ENDPOINT" --api-token "$BUGSINK_API_TOKEN"
```

## Environment Variables
The CLI checks the environment for credentials when no keystore account is specified:
- `BUGSINK_ENDPOINT`: Base URL of the Bugsink server.
- `BUGSINK_API_TOKEN`: 40-character Bearer token.

## Multi-Account Management
Switch or verify accounts using:
```bash
bugsink accounts list --check
bugsink accounts test [account_name]
```

## Error Handling
When authentication fails, commands exit with non-zero exit codes and output standardized JSON/YAML error envelopes on stderr:
- `auth_required` (exit code 3): No token provided or token invalid/rejected by the server.
- `no_account` (exit code 7): Specified account does not exist in keystore.
- `invalid_input` (exit code 6): Bad URL or argument format.
- `rate_limited` (exit code 5): Server rate limits reached.

## Security Best Practices
1. **Never Commit Tokens**: Keep `.env` or plaintext token files out of version control.
2. **Use OS Keystore**: The CLI automatically utilizes macOS Keychain, Windows DPAPI, or Linux Secret Service (`secret-tool`).
3. **Machine Verification**: When writing agent automation scripts, always pass `--json` to reliably capture error codes.
