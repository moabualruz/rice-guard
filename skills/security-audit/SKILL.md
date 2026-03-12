# rice-guard security-audit — AI Agent Skill

## Name
`rice-guard-security-audit` — Security-focused scanning and remediation workflow.

## Description
Combines `scan --security` with `fix --security` for a complete security audit
workflow. Identifies vulnerabilities, secrets, dependency CVEs, and IaC
misconfigurations, then applies deterministic fixes where possible.

## When to Use
- **Security review**: Audit a codebase for vulnerabilities
- **Dependency audit**: Check for known CVEs in dependencies
- **Secret detection**: Find leaked secrets in code and git history
- **Compliance check**: Pre-release security gate

## Prerequisites
- `rice-guard` binary installed
- `.riceguard.yaml` config exists
- Security scanners: Semgrep, Trivy, Gitleaks

## Workflow

### Step 1: Security scan
```bash
rice-guard scan . --security
```
Runs Semgrep (SAST) + Trivy (CVEs, secrets, IaC) + Gitleaks (git history).

### Step 2: Triage findings
```bash
# View summary
cat reports/latest/summary.txt

# High-priority issues (WSJF score > 70)
cat reports/latest/issues.json | jq '[.[] | select(.priority_score > 70)]'

# Auto-fixable security issues
cat reports/latest/issues-fixable.json | jq '[.[] | select(.category == "security")]'
```

### Step 3: Apply deterministic fixes
```bash
rice-guard fix . --security          # Semgrep --autofix rules
rice-guard fix . --deps              # Dependency CVE updates
```

### Step 4: Review remaining
```bash
cat reports/latest/issues-remaining.json
```
Feed remaining issues to AI for assisted remediation — evidence is pre-embedded.

## Interpreting Findings

### Severity Levels
| Level | Meaning | Action |
|-------|---------|--------|
| `error` | Critical vulnerability | Fix immediately |
| `warning` | Moderate risk | Fix before release |
| `info` | Low risk / informational | Review and decide |

### WSJF Priority Score (0-100)
Higher score = fix first. Components:
- **Severity** (40%): error=40, warning=20, info=5
- **Auto-fixable** (20%): yes=20, no=0
- **Category** (20%): security=20, quality=10, style=5
- **File frequency** (10%): issues in hot files score higher
- **Cross-file penalty** (-10%): multi-file fixes are harder

### Categories
| Category | Scanner | Examples |
|----------|---------|----------|
| `security` | Semgrep | SQL injection, XSS, path traversal |
| `vulnerability` | Trivy | Known CVEs in dependencies |
| `secret` | Gitleaks/Trivy | API keys, tokens, passwords |
| `iac` | Trivy | Dockerfile, Terraform misconfigs |

## Automated Remediation

~15-20% of Semgrep security rules have `--autofix` patterns:
- Parameterized query rewrites
- Secure header additions
- Input sanitization wrappers

Dependency fixes via `--deps`:
- `npm audit fix` (semver-safe)
- `pip-audit --fix`
- `cargo update` (semver-safe)

## Manual Review (AI-Assisted)

`issues-remaining.json` contains issues that need human or AI intervention.
Each issue has pre-embedded evidence:

```json
{
  "evidence": {
    "matched_code": "...",
    "context_before": ["..."],
    "context_after": ["..."],
    "enclosing_function": "handle_request",
    "imports": ["use std::fs;"]
  }
}
```

AI agents can use this evidence to generate fixes without reading source files.

## MCP Integration

For automated security workflows via MCP server (`rice-guard mcp`):

```
1. scan_project(security=true)     → get findings
2. get_issues(category="security") → filter security issues
3. fix_issue(id="SEMGREP-042")     → fix specific issue
4. verify_fix(id="SEMGREP-042")    → confirm fix works
5. get_remaining_issues()          → what's left for manual review
```

## Exit Codes
- `0` — No security issues found
- `1` — Security issues found
- `2` — Scanner error
