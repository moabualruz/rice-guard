# rice-guard fix — AI Agent Skill

## Name
`rice-guard-fix` — Deterministic auto-fixing for code quality issues.

## Description
Runs only deterministic fixers — zero AI, zero tokens. Applies formatters,
linter auto-fixes, security patches, AST rewrites, and dependency updates
in a safe, ordered pipeline.

## When to Use
- **After scan**: Fix auto-fixable issues found by `rice-guard scan`
- **CI fix step**: Automated remediation in CI pipelines
- **Pre-commit**: Quick formatting + linting before committing
- **Targeted fix**: Fix a specific issue by ID

## Prerequisites
- `rice-guard` binary installed
- `.riceguard.yaml` config exists
- Fixer tools installed (rustfmt, clippy, ruff, biome, etc.)

## Commands

### Fix all (run all enabled fixer stages)
```bash
rice-guard fix [path]
```

### Fix by category
```bash
rice-guard fix [path] --formatters    # Formatters only
rice-guard fix [path] --linters       # Linter auto-fix only
rice-guard fix [path] --security      # Semgrep --autofix only
rice-guard fix [path] --ast           # ast-grep fix rules only
rice-guard fix [path] --deps          # Dependency updates only
rice-guard fix [path] --imports       # Import cleanup only
```

### Preview fixes (no changes applied)
```bash
rice-guard fix [path] --dry-run
```

### Fix specific issues
```bash
rice-guard fix [path] --issue <ID>           # Fix one issue by ID
rice-guard fix [path] --issues issues.json   # Fix issues from a file
```

### Include unsafe fixes (requires confirmation)
```bash
rice-guard fix [path] --unsafe --yes    # Skip confirmation (CI)
```

### Additional flags
```bash
rice-guard fix [path] --diff            # Include diffs in report
rice-guard fix [path] --rescan          # Re-check after fixing
rice-guard fix [path] --timeout 300     # Set pipeline timeout (seconds)
rice-guard fix [path] file1.rs file2.rs # Fix specific files only
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

1. `rice-guard scan .` — find all issues
2. `rice-guard fix .` — apply deterministic fixes
3. `rice-guard fix . --rescan` — or re-scan to verify
4. Review `issues-remaining.json` for manual/AI fixes

## MCP Integration

| Tool | Description |
|------|-------------|
| `fix_all` | Run all fixer stages |
| `fix_issue` | Fix a specific issue by ID |
| `fix_preview` | Preview fixes without applying (dry-run) |
| `get_fix_report` | Get the latest fix report |
| `verify_fix` | Re-check a specific issue after fixing |

## Exit Codes
- `0` — All fixes applied successfully (or dry-run)
- `1` — Some fixes failed
- `2` — Tool error (config missing, fixer not found)
