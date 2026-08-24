# Repository Guidelines

## Project Structure & Module Organization

TokenGlass is a Tauri 2 desktop app with a React/TypeScript frontend and Rust backend.

- `src/`: React UI. `App.tsx` is the dashboard, `Widget.tsx` is the floating widget, and `usage.ts` holds shared usage types and formatting logic.
- `src-tauri/src/lib.rs`: Tauri commands, API integration, tray/window behavior, and Rust unit tests. `src-tauri/capabilities/` defines Tauri permissions.
- `scripts/`: sidecar preparation and Windows test-build automation.
- `docs/`: manual verification guides; `openspec/` contains active specifications, approved main specs, and archived changes.

Keep UI behavior shared between the dashboard and widget in common TypeScript modules rather than duplicating it.

## Build, Test, and Development Commands

- `pnpm install --frozen-lockfile`: install locked frontend dependencies.
- `pnpm dev`: run the Vite frontend only.
- `pnpm tauri dev`: run the full native desktop application.
- `pnpm build`: prepare the Codex sidecar, type-check TypeScript, and build the frontend.
- `cargo test --manifest-path src-tauri/Cargo.toml`: run Rust unit tests.
- `pnpm test:windows-support`: run the focused diagnostics test suite.
- `pnpm format`: format frontend files with Biome and Rust with `cargo fmt`.
- `pnpm build:windows-test` then `pnpm smoke:windows`: build and smoke-test the Windows x64 test-mode package.

## Coding Style & Naming Conventions

Biome uses two-space indentation and a 100-character line width. Run formatting on changed files before submitting. Use PascalCase for React components, camelCase for TypeScript values/functions, and Rust `snake_case`. Keep Tauri command payloads serialized in camelCase with `#[serde(rename_all = "camelCase")]`.

## Testing Guidelines

Add Rust tests beside related backend behavior in `src-tauri/src/lib.rs`; use descriptive scenario-oriented test names. At minimum, run `pnpm build` and relevant `cargo test` commands. For window, tray, widget, OAuth, or scaling changes, manually verify with `pnpm tauri dev`; follow [docs/windows-x64-test.md](docs/windows-x64-test.md) for Windows acceptance checks.

## Commits, Pull Requests, and Security

Use concise Conventional Commit-style messages, optionally scoped: `feat(openai-usage): share refresh cache`, `refactor(tauri): clean up widget setup`. PRs should summarize user-visible changes, link issues when applicable, list commands run, and include screenshots for UI changes. Never commit API keys, OAuth tokens, authorization headers, generated sidecar binaries, or copied diagnostics containing secrets.

For behavior changes, create or update an OpenSpec change before implementation; archive it only after tasks, validation, and main-spec synchronization are complete.
