# rice-guard — Agent Instructions

This file follows the [AGENTS.md standard](https://agents.md/) for cross-tool
AI agent context. It is consumed by Codex, Jules, Amp, Gemini CLI, Cursor,
and any agent that supports the convention.

## Shared Instructions

Full project context is in `.ai/instructions/`:

- `.ai/instructions/project.md` — Project summary, current state
- `.ai/instructions/architecture.md` — Tech stack, scanners, fixers, workspace
- `.ai/instructions/cli-reference.md` — CLI subcommands, output files
- `.ai/instructions/conventions.md` — Style, build commands, branding
- `.ai/instructions/phases.md` — Phase status tracker
- `.ai/instructions/agent-config.md` — Agent setup documentation

Agents that support `@` imports (Claude Code, Gemini CLI) load these
automatically via `CLAUDE.md` / `GEMINI.md`. Agents that don't should
read the files listed above directly.

## Quick Reference

- **Name**: rice-guard
- **Language**: Rust (edition 2021, stable toolchain)
- **Type**: Cross-platform CLI for automated code quality + security scanning
- **Config file**: `.riceguard.yaml`

```bash
cargo check                        # Type-check all crates
cargo build                        # Debug build
cargo test                         # Run all tests
cargo fmt --all                    # Format (run before every commit)
cargo clippy -- -D warnings        # Lint (must pass clean)
```
