## 1. Cursor-page retrieval

- [x] 1.1 Add sequential Usage and Costs cursor traversal that preserves fixed query bounds and endpoint filters; verify with Rust unit tests covering multiple pages.
- [x] 1.2 Reject missing or repeated continuation cursors without producing a result; verify with Rust unit tests for both malformed response shapes.

## 2. Aggregation and cache integration

- [x] 2.1 Aggregate every Usage page's model token totals and every Costs page's amounts while enforcing one currency; verify with a multi-page aggregate test.
- [x] 2.2 Route the existing usage refresh through complete page aggregation so a failure leaves the current cache unchanged; verify with `cargo test --manifest-path src-tauri/Cargo.toml`.

## 3. Validation

- [x] 3.1 Format changed files and verify `pnpm build`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `openspec validate add-openai-usage-pagination --strict` pass.
- [ ] 3.2 Manually inspect test-mode dashboard and widget data to confirm existing USD/KRW and refresh status rendering still works.
