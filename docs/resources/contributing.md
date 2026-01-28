# Contributing to Portal

We welcome contributions to Portal! This guide will help you get started.

## Ways to Contribute

- **Report bugs** - Open an issue on GitHub
- **Suggest features** - Share your ideas
- **Improve documentation** - Fix typos, add examples
- **Submit code** - Fix bugs or implement features
- **Answer questions** - Help others in discussions

## Development Setup

1. **Fork the repository**
```bash
git clone https://github.com/PortalTechnologiesInc/lib.git
cd lib
```

2. **Set up development environment**
```bash
# Using Nix (recommended)
nix develop

# Or manually with Cargo
cargo build
```

3. **Run tests**
```bash
cargo test
```

4. **Make your changes**

5. **Run linting**
```bash
cargo fmt
cargo clippy
```

6. **Submit a pull request**

## Code Style

- Follow Rust conventions
- Use `cargo fmt` before committing
- Fix `cargo clippy` warnings
- Write tests for new features

## Documentation

When adding features:
- Update relevant documentation
- Add code examples
- Update the changelog

## Questions?

- Open a GitHub issue
- Join community discussions
- Read the existing documentation

---

**Thank you for contributing to Portal!**

