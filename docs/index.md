# Bugsink CLI

> A blazing fast native command-line tool and agent interface for the self-hosted Bugsink error tracking platform, written in Rust 2024.

## Quick Install

```bash
cargo install --git https://github.com/SpaceCorps/Bugsink-Cli --locked
```

## Features

- **Native Speed**: Compiled with Rust 2024, LTO, and zero async runtime bloat. Executes in 1 to 3 ms with minimal resident memory.
- **Prompt Injection Shield**: Treats all crash traces, issue comments, and exception messages strictly as untrusted data, protecting agent loops.
- **OS Keystores**: Saves tokens in macOS Keychain, Windows DPAPI, or Linux Secret Service (libsecret).
- **Deterministic Multi-Account**: Explicit `--account <name>` (short `-a`) scoping prevents accidental cross-environment destruction.
- **Agentic Output Protocol**: Clean YAML by default, raw JSON with `--json`, structured stderr error envelopes, stable numeric exit codes.
- **Self-Documenting**: Embedded `bugsink agent-readme [--json]` command for instant zero-network agent discovery.

## Documentation & Links

- [Documentation Website](https://spacecorps.github.io/Bugsink-Cli/)
- [Agent Manual (llms.txt)](https://spacecorps.github.io/Bugsink-Cli/llms.txt)
- [Comprehensive Technical Manual (llms-full.txt)](https://spacecorps.github.io/Bugsink-Cli/llms-full.txt)
- [Authentication Guide](https://spacecorps.github.io/Bugsink-Cli/auth.md)
- [About & History](https://spacecorps.github.io/Bugsink-Cli/about.html)
- [Pricing & Open Source](https://spacecorps.github.io/Bugsink-Cli/pricing.md)
- [Privacy Policy](https://spacecorps.github.io/Bugsink-Cli/privacy.html)
- [GitHub Repository](https://github.com/SpaceCorps/Bugsink-Cli)
