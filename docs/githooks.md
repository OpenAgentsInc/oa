# Git Hooks

This project uses Git hooks to maintain code quality and consistency. The hooks are stored in the `.githooks` directory and need to be configured locally.

## Setup

To enable the git hooks, run these commands in your local repository:

```bash
# Make the pre-commit hook executable
chmod +x .githooks/pre-commit

# Configure git to use the .githooks directory
git config core.hooksPath .githooks
```

## Available Hooks

### Pre-commit Hook

The pre-commit hook automatically formats Rust code using `cargo fmt` before each commit. Here's how it works:

1. When you attempt to commit, the hook runs `cargo fmt --check` to detect any formatting issues
2. If formatting is needed:
   - The hook runs `cargo fmt` to fix the formatting
   - Stages the formatted files
   - Allows the commit to proceed
3. If no formatting is needed, the commit proceeds normally

This ensures that all committed code follows the standard Rust formatting guidelines.