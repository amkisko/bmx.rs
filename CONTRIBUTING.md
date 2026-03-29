# Contributing

Thank you for contributing to bmx.

## Contribution Principles

Contributions should be clear, testable, and scoped to one logical change. Each pull request should explain why the change is needed, what behavior changes, and how it was validated.

Contributors are responsible for all submitted output, including chatbot-assisted output. Every change should be reviewed and understood before submission. Secrets, credentials, and private data must never be included in code, prompts, logs, or documentation.

## Development Workflow

Before opening a pull request, run the standard checks locally:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

If any check fails, resolve the issue before submitting.

## Reporting Issues

Issue reports should be reproducible and specific. Include the expected behavior, the actual behavior, clear reproduction steps, and relevant environment details.
