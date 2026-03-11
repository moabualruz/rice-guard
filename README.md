# rice-guard

One CLI for code quality, security scanning, and deterministic auto-fixing.

```bash
rice-guard init .     # detect languages + tools, generate config
rice-guard scan .     # run all scanners in parallel
rice-guard fix .      # auto-fix everything deterministic (zero AI, zero cost)
rice-guard status .   # see what's left
```

## What It Does

- **Scan** — orchestrates Semgrep, Trivy, Gitleaks, jscpd, and scc in parallel
- **Fix** — runs formatters, linter `--fix`, Semgrep `--autofix`, ast-grep rewrites, and dep updates
- **Report** — unfixed issues get AI-ready output with embedded code evidence, so any AI tool can consume them

Every fix is deterministic. No LLM calls, no API keys, no tokens.

## Install

```bash
cargo install rice-guard           # from crates.io
brew install rice-guard            # macOS / Linux
scoop install rice-guard           # Windows
npm install -g rice-guard          # npm wrapper
```

## Supported Languages

Go, Java, PHP, Kotlin, Python, Rust, Dart/Flutter, TypeScript, JavaScript, C#, Ruby, Shell, and more.

## Status

Under active development.

## License

[CC BY-NC-SA 4.0](LICENSE.md) — free for non-commercial use.
Commercial use requires a separate license.
