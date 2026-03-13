# rguard fix — AI Agent Skill

## Name

`rguard-fix` — Deterministic auto-fixing for code quality issues.

## Description

Runs only deterministic fixers — zero AI, zero tokens. Applies formatters,
linter auto-fixes, security patches, AST rewrites, and dependency updates
in a safe, ordered pipeline.

## When to Use

- **After scan**: Fix auto-fixable issues found by `rguard scan`
- **CI fix step**: Automated remediation in CI pipelines
- **Pre-commit**: Quick formatting + linting before committing
- **Targeted fix**: Fix a specific issue by ID

## Prerequisites

- `rguard` binary installed
- `.rguard.yaml` config exists
- Fixer tools installed (rustfmt, clippy, ruff, biome, etc.)

## Commands

### Fix all (run all enabled fixer stages)

```bash
rguard fix [path]
```

### Fix by category

```bash
rguard fix [path] --formatters    # Formatters only
rguard fix [path] --linters       # Linter auto-fix only
rguard fix [path] --security      # Semgrep --autofix only
rguard fix [path] --ast           # ast-grep fix rules only
rguard fix [path] --deps          # Dependency updates only
rguard fix [path] --imports       # Import cleanup only
```

### Preview fixes (no changes applied)

```bash
rguard fix [path] --dry-run
```

### Fix specific issues

```bash
rguard fix [path] --issue <ID>           # Fix one issue by ID
rguard fix [path] --issues issues.json   # Fix issues from a file
```

### Include unsafe fixes (requires confirmation)

```bash
rguard fix [path] --unsafe --yes    # Skip confirmation (CI)
```

### Additional flags

```bash
rguard fix [path] --diff            # Include diffs in report
rguard fix [path] --rescan          # Re-check after fixing
rguard fix [path] --timeout 300     # Set pipeline timeout (seconds)
rguard fix [path] file1.rs file2.rs # Fix specific files only
```

## Fix Pipeline Order

1. **Formatters** — gofumpt, rustfmt, ruff format, biome, prettier, dart format
2. **Linters** — golangci-lint --fix, cargo clippy --fix, ruff check --fix
3. **Security** — semgrep --autofix (~15-20% of rules)
4. **AST** — ast-grep fix rules (embedded, sub-ms)
5. **Deps** — npm audit fix, pip-audit --fix, cargo update
6. **Imports** — goimports, ruff F401+I, biome, dart fix

Each stage: check → fix → verify → record.

## Output

`fix-report.json` written to `reports/<project>/<timestamp>/`:

```json
{
  "schema_version": "1.0",
  "dry_run": false,
  "stages": [
    {
      "name": "formatters",
      "tool": "rustfmt",
      "files_changed": 3,
      "status": "applied"
    }
  ],
  "total_fixed": 12,
  "total_skipped": 3,
  "duration_ms": 1500
}
```

## The Fix Loop

```
scan → fix → rescan → verify
```

1. `rguard scan .` — find all issues
2. `rguard fix .` — apply deterministic fixes
3. `rguard fix . --rescan` — or re-scan to verify
4. Review `issues-remaining.json` for manual/AI fixes

## MCP Integration

| Tool             | Description                              |
| ---------------- | ---------------------------------------- |
| `fix_all`        | Run all fixer stages                     |
| `fix_issue`      | Fix a specific issue by ID               |
| `fix_preview`    | Preview fixes without applying (dry-run) |
| `get_fix_report` | Get the latest fix report                |
| `verify_fix`     | Re-check a specific issue after fixing   |

## Exit Codes

- `0` — All fixes applied successfully (or dry-run)
- `1` — Some fixes failed
- `2` — Tool error (config missing, fixer not found)
