# Git Hooks

This project uses Git hooks to maintain code quality and consistency. The hooks are stored in the `.githooks` directory and need to be configured locally.

## Setup

To enable the git hooks, run these commands in your local repository:

```bash
# Make the pre-commit hook executable
chmod +x .githooks/pre-commit

# Configure git to use the .githooks directory
git config core.hooksPath .githooks

# Install cargo-deny for security checks (optional but recommended)
cargo install cargo-deny
```

## Available Hooks

### Pre-commit Hook

The pre-commit hook runs several checks before allowing a commit:

1. **Code Formatting** (`cargo fmt`)
   - Checks if code needs formatting
   - Automatically formats and stages any unformatted code

2. **Linting** (`cargo clippy`)
   - Runs clippy with warnings as errors
   - Fails if any linting issues are found

3. **Tests** (`cargo test`)
   - Runs the full test suite
   - Fails if any tests fail

4. **Security Audit** (`cargo deny`)
   - If cargo-deny is installed, checks for known security advisories
   - Skipped if cargo-deny is not installed (with a warning)

The hook will prevent commits if any checks fail (except formatting, which is auto-fixed). This ensures that all committed code:
- Follows standard Rust formatting
- Passes all linting checks
- Has passing tests
- Is free from known security vulnerabilities

All output is color-coded for better visibility.