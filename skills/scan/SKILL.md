# rice-guard scan — AI Agent Skill

## Name

`rice-guard-scan` — Code quality and security scanning with AI-ready output.

## Description

Scans a project for code quality, security, duplication, and complexity issues.
Produces structured JSON reports with embedded code evidence that AI agents can
consume directly — no need to read source files.

## When to Use

- **Code review**: Scan before or during PR review to find issues
- **Quality gate**: Run in CI to enforce quality standards
- **Security audit**: Use `--security` for focused security scanning
- **Baseline check**: Initial scan of a new codebase

## Prerequisites

- `rice-guard` binary installed
- `.riceguard.yaml` config exists (run `rice-guard init` first)
- External scanners installed as needed (Semgrep, Trivy, Gitleaks, etc.)

## Commands

### Full scan (all enabled scanners)

```bash
rice-guard scan [path]
```

### Quick scan (jscpd + scc + Semgrep + Trivy)

```bash
rice-guard scan [path] --quick
```

### Security-focused scan (Semgrep + Trivy + Gitleaks)

```bash
rice-guard scan [path] --security
```

### Hold-the-line (new issues only vs baseline)

```bash
rice-guard scan [path] --diff-only
```

## Output Files

All reports written to `reports/<project>/<timestamp>/`:

| File                    | Contents                                   |
| ----------------------- | ------------------------------------------ |
| `issues.json`           | All issues, priority-sorted, with evidence |
| `issues-fixable.json`   | Issues with deterministic fixes available  |
| `issues-remaining.json` | Issues needing manual or AI intervention   |
| `summary.json`          | Scan summary + counts                      |
| `summary.txt`           | Human-readable table                       |
| `*.sarif`               | Raw SARIF per scanner                      |

## Issue JSON Schema

Each issue in `issues.json` includes:

```json
{
  "id": "SEMGREP-001",
  "scanner": "semgrep",
  "severity": "error",
  "category": "security",
  "message": "SQL injection vulnerability",
  "file": "src/db.rs",
  "line": 42,
  "evidence": {
    "matched_code": "query(&format!(\"SELECT * FROM {} WHERE id = {}\", table, id))",
    "context_before": ["fn get_user(id: i64) {"],
    "context_after": ["}"],
    "enclosing_function": "get_user",
    "enclosing_class": null,
    "imports": ["use sqlx::query;"]
  },
  "fix": {
    "auto_fixable": false,
    "auto_fix_tool": null,
    "suggested_replacement": null,
    "complexity": "moderate"
  },
  "priority_score": 85
}
```

## Parsing Output

```bash
# Count issues by severity
cat reports/latest/issues.json | jq '[.[] | .severity] | group_by(.) | map({(.[0]): length}) | add'

# Get high-priority fixable issues
cat reports/latest/issues-fixable.json | jq '[.[] | select(.priority_score > 70)]'

# List files with most issues
cat reports/latest/issues.json | jq '[.[] | .file] | group_by(.) | map({file: .[0], count: length}) | sort_by(-.count)'
```

## MCP Integration

When using rice-guard as an MCP server (`rice-guard mcp`):

| Tool                   | Description                          |
| ---------------------- | ------------------------------------ |
| `scan_project`         | Run a full scan and return issues    |
| `get_issues`           | Get issues from the latest scan      |
| `get_issue`            | Get a specific issue by ID           |
| `get_remaining_issues` | Get issues that need manual/AI fixes |

## Exit Codes

- `0` — Clean, no issues found
- `1` — Issues found (scan succeeded)
- `2` — Tool error (scanner failed, config missing)
