<p align="center">
  <a href="https://github.com/moabualruz/rice-guard">
    <img src="https://raw.githubusercontent.com/moabualruz/rice-guard/main/.branding/banner.png" alt="riceGuard" width="500" />
  </a>
</p>

<p align="center">
  <strong>One CLI for code quality, security scanning, and deterministic auto-fixing.</strong>
</p>

<p align="center">
  <a href="https://github.com/moabualruz/rice-guard/actions"><img src="https://img.shields.io/github/actions/workflow/status/moabualruz/rice-guard/ci.yml?branch=main&style=flat-square&label=CI" alt="CI Status" /></a>
  <a href="https://crates.io/crates/rguard"><img src="https://img.shields.io/crates/v/rguard?style=flat-square&color=00a020" alt="Crates.io" /></a>
  <a href="https://github.com/moabualruz/rice-guard/blob/main/LICENSE.md"><img src="https://img.shields.io/badge/license-CC%20BY--NC--SA%204.0-00a020?style=flat-square" alt="License" /></a>
  <a href="https://github.com/moabualruz/rice-guard/releases"><img src="https://img.shields.io/github/v/release/moabualruz/rice-guard?style=flat-square&color=00a020" alt="Latest Release" /></a>
</p>

---

**riceGuard** orchestrates best-in-class scanners and fixers into a single, fast command-line tool. It detects languages automatically, runs scanners in parallel, applies deterministic fixes with zero AI or API keys, and produces AI-ready reports for everything it can't fix — so any LLM can pick up where the tooling left off.

## Table of Contents

- [Why riceGuard](#why-riceguard)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [How It Works](#how-it-works)
- [Supported Languages](#supported-languages)
- [Scanner Stack](#scanner-stack)
- [Fix Pipeline](#fix-pipeline)
- [Configuration](#configuration)
- [AI-Ready Reports](#ai-ready-reports)
- [Integrations](#integrations)
- [License](#license)

## Why riceGuard

Most teams cobble together 5-10 separate tools for linting, formatting, security scanning, and dependency auditing — each with its own config, CLI, and CI step. **riceGuard** replaces that with one binary:

- **Single command** — `rguard scan .` runs Semgrep, Trivy, Gitleaks, jscpd, and scc in parallel
- **Deterministic fixes** — `rguard fix .` applies formatters, linter `--fix`, Semgrep `--autofix`, ast-grep rewrites, and dependency updates. No LLM calls, no API keys, no tokens, no cost
- **AI-ready output** — unfixed issues include embedded source evidence (matched code, enclosing function, imports) so any AI tool can consume them without reading your files
- **Polyglot** — one config file covers Go, Java, Python, Rust, TypeScript, Kotlin, Dart, PHP, C#, Ruby, and more
- **Fast** — native Rust binary with embedded ast-grep and tree-sitter. Sub-10ms startup, parallel execution

## Quick Start

```bash
# Install
cargo install rguard

# Set up your project (auto-detects languages and available tools)
rguard init .

# Scan for issues
rguard scan .

# Auto-fix everything deterministic
rguard fix .

# See what's left
rguard status .
```

<p align="center">
  <img src="https://raw.githubusercontent.com/moabualruz/rice-guard/main/.branding/logo.png" alt="rG" width="80" />
</p>

## Installation

### From Source (Rust)

```bash
cargo install rguard
```

### Homebrew (macOS / Linux)

```bash
brew install rguard
```

### Scoop (Windows)

```powershell
scoop install rguard
```

### npm Wrapper

```bash
npm install -g rguard
```

### Shell Script (Linux / macOS)

```bash
curl -fsSL https://install.rguard.dev | bash
```

### PowerShell (Windows)

```powershell
irm https://install.rguard.dev/windows | iex
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/moabualruz/rice-guard/releases) — available for Linux, macOS, and Windows on x64 and ARM64.

## How It Works

```
rguard init .     ──►  Detect languages (scc) + probe available tools
                       Generate .rguard.yaml with only what's installed

rguard scan .     ──►  Run all enabled scanners in parallel
                       Merge results into priority-sorted issues
                       Embed source evidence via ast-grep + tree-sitter

rguard fix .      ──►  Formatters → Linters → Security → AST → Deps → Imports
                       Each stage: check → fix → verify → record
                       Zero AI. Every fix is deterministic and repeatable.

rguard status .   ──►  Issue counts, trend tracking, what's left to fix
```

### Init Detection Pipeline

`rguard init` uses a 3-step pipeline:

1. **Language detection** — `scc` scans the project tree for language breakdown (files, LOC, complexity)
2. **Scanner probing** — checks each scanner's availability via YAML plugin descriptors
3. **Fixer probing** — for each detected language, probes every fixer tool

The generated `.rguard.yaml` enables only tools that are actually installed, with version-stamped comments and install hints for missing tools.

## Supported Languages

| Tier | Languages |
|------|-----------|
| **High priority** | Go, Java, PHP, Kotlin, Python, Rust, Dart/Flutter, TypeScript, JavaScript |
| **Supported** | C#, Ruby, Shell, YAML, JSON, HTML, CSS, Markdown, Dockerfile, Terraform |

Language support is determined by the scanner and fixer tools available on your system. riceGuard detects what's installed and adapts.

## Scanner Stack

| Tool | Role | Integration |
|------|------|-------------|
| **ast-grep** | AST pattern search + rewrite + evidence extraction | Embedded (Rust crate, sub-ms) |
| **tree-sitter** | Parse trees for enclosing function/class extraction | Embedded (Rust crate) |
| **Semgrep OSS** | SAST security + custom architecture rules | Subprocess |
| **Trivy** | Dependency CVEs, secrets, IaC misconfig, licenses, SBOM | Subprocess |
| **Gitleaks** | Git history secret scanning | Subprocess |
| **jscpd** | Code duplication detection | Subprocess |
| **scc** | Lines of code, complexity metrics, language detection | Subprocess |

External tools are orchestrated via YAML plugin descriptors — no Rust code changes needed to add new scanners.

## Fix Pipeline

Every fix is deterministic. No AI, no LLM calls, no API keys.

| Stage | Tools | Safety |
|-------|-------|--------|
| **Formatters** | gofumpt, rustfmt, ruff format, dart format, biome, prettier, etc. | Always safe |
| **Linter --fix** | golangci-lint --fix, cargo clippy --fix, ruff check --fix, etc. | Safe + unsafe tiers |
| **Security** | Semgrep --autofix (~15-20% of rules have auto-fix patterns) | Pattern-based |
| **AST rewrites** | ast-grep fix rules (embedded, sub-millisecond) | Custom rules |
| **Dep updates** | npm audit fix, pip-audit --fix, cargo update, go mod tidy | Semver-safe |
| **Import cleanup** | goimports, ruff F401+I, biome, dart fix, dotnet format | Safe |

Execution order is fixed: formatters → linters → security → AST → deps → imports. Each stage runs check → fix → verify → record.

```bash
rguard fix .               # Run ALL deterministic fixers
rguard fix . --formatters  # Formatters only
rguard fix . --linters     # Linter auto-fix only
rguard fix . --security    # Semgrep --autofix only
rguard fix . --ast         # ast-grep fix rules only
rguard fix . --deps        # Dependency updates only
rguard fix . --dry-run     # Preview fixes without applying
rguard fix . --unsafe      # Include unsafe linter fixes
```

## Configuration

riceGuard uses a single `.rguard.yaml` file at the project root. Generate one with:

```bash
rguard init .         # Interactive setup
rguard init . --yes   # Non-interactive, auto-detect defaults
```

### File Exclusion

Create a `.rguardignore` or `.rgignore` file with gitignore-compatible patterns:

```gitignore
# Exclude generated code
*.generated.*
*.pb.go

# Exclude vendored dependencies
vendor/
third_party/
```

Patterns from multiple sources are merged additively:

1. Built-in defaults (vendor/, node_modules/, target/, .git/, dist/)
2. Global ignore (`~/.config/rguard/ignore`)
3. `.gitignore` (opt-in via `respect_gitignore: true` in config)
4. `.rguardignore` / `.rgignore`
5. `filters.exclude` in `.rguard.yaml`

Debug unexpected exclusions:

```bash
rguard scan . --debug-ignores
```

## AI-Ready Reports

Every finding includes pre-embedded source evidence so AI agents can fix issues without reading your files:

```json
{
  "id": "SEC-001",
  "evidence": {
    "matched_code": "password = request.form['password']",
    "context_before": ["@app.route('/login', methods=['POST'])"],
    "context_after": ["db.execute(f'SELECT * FROM users WHERE pw={password}')"],
    "enclosing_function": "def login():",
    "enclosing_class": null,
    "imports": ["from flask import Flask, request"]
  },
  "fix": {
    "auto_fixable": false,
    "complexity": "moderate",
    "suggested_replacement": "Use parameterized queries",
    "verification": { "rerun_command": "rguard scan . --security" }
  },
  "priority_score": 85
}
```

Reports are compatible with Claude, GPT, Gemini, Cursor, Copilot, and any tool that reads JSON.

### Output Files

```
reports/<project>/<timestamp>/
├── issues.json              # All issues, priority-sorted, with evidence
├── issues-fixable.json      # Issues with deterministic fixes available
├── issues-remaining.json    # Issues that need manual or AI intervention
├── fix-report.json          # What was fixed (after running fix)
├── summary.json             # Scan summary + counts
├── summary.txt              # Human-readable table
└── *.sarif                  # Raw SARIF per scanner
```

## Integrations

### CI/CD

riceGuard generates CI config during init:

```bash
rguard init .   # Generates GitHub Actions / GitLab CI templates
```

### Pre-commit Hooks

Generates [Lefthook](https://github.com/evilmartians/lefthook) configuration:

```yaml
pre-commit:
  commands:
    rguard-scan:
      run: rguard scan . --quick
      fail_text: "rguard found issues. Run: rguard fix ."
```

### REST API

```bash
rguard serve --port 8080
```

### MCP Server (AI Agents)

riceGuard includes a built-in MCP server for AI coding agents:

```bash
rguard serve --mcp   # stdio JSON-RPC for Claude, Cursor, etc.
```

### SonarQube

Optional integration for teams using SonarQube dashboards:

```bash
rguard enroll .   # Add project to SonarQube
rguard report .   # Pull quality gate report
```

## Architecture

```
crates/cli/          # Binary: clap + main
crates/core/         # Library: scanner engine, fixer engine, evidence, issues
crates/server/       # Library: axum REST API + MCP server
descriptors/
  scanners/          # Scanner plugin YAML configs
  fixers/            # Fixer plugin YAML configs (per-language)
templates/           # CI/CD and config templates
```

- **Rust** — native ast-grep/tree-sitter, serde, < 10ms startup, single binary
- **tokio** — async scanner/fixer parallelism
- **rayon** — CPU-bound AST work
- **axum** — REST API server
- **clap v4** — CLI with derive macros
- **YAML plugin system** — add scanners/fixers without Rust code changes

## Contributing

```bash
cargo check                        # Type-check
cargo build                        # Debug build
cargo test                         # Run all tests
cargo fmt --all                    # Format (required before commit)
cargo clippy -- -D warnings        # Lint (must pass clean)
```

## License

[CC BY-NC-SA 4.0](LICENSE.md) — free for non-commercial use. Commercial use requires a separate license.

<p align="center">
  <br />
  <a href="https://github.com/moabualruz/rice-guard">
    <img src="https://raw.githubusercontent.com/moabualruz/rice-guard/main/.branding/mini.logo.png" alt="rG" width="40" />
  </a>
  <br />
  <sub>Built with Rust. Zero AI in the fix pipeline.</sub>
</p>
