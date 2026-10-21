## 1. Intel macOS sidecar preparation

- [x] 1.1 Add Intel macOS target selection to the Node preflight and POSIX installer, then verify the target resolver test passes.
- [x] 1.2 Recognize and normalize the Intel macOS installer output as `codex-x86_64-apple-darwin`, then verify the shell script syntax check passes.

## 2. Regression coverage and documentation

- [x] 2.1 Add target-selection tests for supported and unsupported platform/architecture pairs, then run them with Node's test runner.
- [x] 2.2 Document Intel macOS automatic sidecar preparation and verify the README matches the supported mapping.

## 3. Integrated verification

- [x] 3.1 Run formatting, OpenSpec strict validation, and the frontend build to verify the complete preparation flow remains valid.
- [x] 3.2 Add an Intel macOS GitHub Actions workflow that prepares the sidecar, verifies the x86_64 target, and builds a Tauri app bundle.
