## Why

The OpenAI Usage API returns at most 31 daily buckets per page, while the current
dashboard reads only the first page. A reporting range longer than one page can
therefore understate token usage and costs without warning.

## What Changes

- Retrieve every Usage and Costs response page for a fixed reporting period.
- Use each endpoint's supported daily page size while preserving the existing
  local-time monthly and daily calculations.
- Reject malformed pagination responses and do not cache partial aggregates.

## Capabilities

### New Capabilities

- `openai-usage-pagination`: Complete, cursor-based aggregation of OpenAI
  organization Usage and Costs time buckets.

### Modified Capabilities

- `openai-usage-refresh`: Cache only a fully retrieved usage snapshot when a
  network refresh succeeds.

## Impact

- `src-tauri/src/lib.rs` OpenAI organization API requests and aggregation.
- Rust regression tests for cursor traversal and malformed page handling.
- The in-process usage cache's network-refresh success boundary.
