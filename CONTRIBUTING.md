# Contributing to SuperNovae CLI

Thank you for your interest in contributing to SuperNovae CLI! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.85 or later (MSRV)
- Linux: `libdbus-1-dev` and `pkg-config` for keyring support
- **Note:** Windows is not yet supported (daemon uses Unix sockets)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/supernovae-st/supernovae-cli
cd supernovae-cli

# Build
cargo build --workspace

# Run tests
cargo test --workspace

# Run clippy (must pass with zero warnings)
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --all
```

## Workspace Structure

```
supernovae-cli/
├── crates/
│   ├── spn-core/      # Shared types, provider definitions
│   ├── spn-keyring/   # OS keychain integration
│   ├── spn-client/    # SDK for external tools
│   ├── spn-providers/ # Cloud backends (Anthropic, OpenAI, etc.)
│   ├── spn-native/    # HuggingFace + mistral.rs inference
│   ├── spn-mcp/       # REST-to-MCP wrapper
│   └── spn/           # Main CLI binary (spn-cli)
```

## Code Style

- **Formatting**: Run `cargo fmt --all` before committing
- **Linting**: Code must pass `cargo clippy --workspace -- -D warnings`
- **Tests**: Add tests for new functionality
- **Documentation**: Document public APIs with rustdoc comments

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

Co-Authored-By: Your Name <email@example.com>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`

Examples:
- `feat(provider): add support for Gemini API`
- `fix(daemon): resolve socket permission issue`
- `docs(readme): update installation instructions`

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Make your changes
4. Ensure all checks pass:
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Commit with a descriptive message
6. Push and create a Pull Request

## Testing

- Unit tests go in the same file as the code (`#[cfg(test)]` module)
- Integration tests go in `tests/` directory
- Aim for 80% coverage on new code

## Security

If you discover a security vulnerability, please see [SECURITY.md](SECURITY.md) for responsible disclosure guidelines.

## Questions?

- Open a [GitHub Discussion](https://github.com/supernovae-st/supernovae-cli/discussions)
- Check existing [Issues](https://github.com/supernovae-st/supernovae-cli/issues)

## License

By contributing, you agree that your contributions will be licensed under the AGPL-3.0 license.
