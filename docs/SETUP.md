# Documentation Setup

This documentation can be compiled and hosted using GitBook or mdBook.

## Using GitBook

### Local Preview

1. **Install GitBook CLI** (legacy version for local preview):
```bash
npm install -g @gitbook/cli
```

2. **Install dependencies**:
```bash
cd docs
gitbook install
```

3. **Serve locally**:
```bash
gitbook serve
```

Visit `http://localhost:4000` to view the documentation.

### Build Static Site

```bash
gitbook build
```

This generates a static website in the `_book/` directory.

## Using mdBook (Alternative)

mdBook is a modern Rust-based alternative that's faster and easier to use.

### Installation

```bash
cargo install mdbook
```

### Convert to mdBook Format

1. **Create mdBook project**:
```bash
mdbook init portal-docs
```

2. **Copy content**:
```bash
cp -r docs/* portal-docs/src/
```

3. **Update SUMMARY.md** to use mdBook format (similar to current format)

4. **Serve**:
```bash
cd portal-docs
mdbook serve
```

Visit `http://localhost:3000` to view the documentation.

### Build

```bash
mdbook build
```

Output will be in `portal-docs/book/`.

## Using GitBook.com (Recommended for Hosting)

1. Create an account at [gitbook.com](https://www.gitbook.com)
2. Create a new space
3. Connect to your GitHub repository
4. GitBook will automatically build and host your docs
5. The `.gitbook.yaml` file configures the build

## File Structure

```
docs/
├── README.md                 # Home page
├── SUMMARY.md               # Table of contents
├── book.json                # GitBook configuration
├── .gitbook.yaml            # GitBook.com configuration
├── introduction/            # Concept explanations
├── getting-started/         # Setup guides
├── sdk/                     # TypeScript SDK docs
├── guides/                  # Feature guides
├── api/                     # API reference
└── resources/               # FAQ, glossary, etc.
```

## Contributing to Docs

1. Edit Markdown files in the `docs/` directory
2. Preview locally before committing
3. Follow the existing structure and style
4. Add new pages to `SUMMARY.md`
5. Submit a pull request

## Documentation Style Guide

- Use clear, concise language
- Include code examples for all features
- Start with basic examples, then show advanced usage
- Use TypeScript for code examples
- Include error handling in examples
- Add "Next Steps" links at the end of guides
- Keep examples self-contained and runnable

---

**Need help?** Open an issue on [GitHub](https://github.com/PortalTechnologiesInc/lib/issues).

