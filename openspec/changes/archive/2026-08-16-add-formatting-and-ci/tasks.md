## 1. Formatting Tooling

- [x] 1.1 Add Biome as a development dependency and create repository formatting configuration with generated-file exclusions.
- [x] 1.2 Add `.editorconfig` rules that align editor defaults with the repository formatter.
- [x] 1.3 Add `pnpm format` and non-mutating `pnpm format:check` commands that cover Biome and `cargo fmt`.

## 2. Baseline and Continuous Integration

- [x] 2.1 Run the formatter across supported existing source and configuration files, preserving intentional generated artifacts.
- [x] 2.2 Add a GitHub Actions workflow that runs the format check for pull requests and pushes to the primary development branches.
- [x] 2.3 Verify the local format check, Rust format check, TypeScript production build, and workflow syntax.
