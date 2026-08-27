## Context

The current OpenAI request path uses a fixed `limit=31` and consumes only one
response from each endpoint. See proposal.md for motivation and the delta specs
for required behavior.

## Goals / Non-Goals

**Goals:**

- Aggregate every cursor page for one immutable local-time reporting period.
- Preserve a single success/failure boundary for the existing shared cache.
- Keep the current dashboard response contract unchanged.

**Non-Goals:**

- Add a period picker, historical report UI, persistence, retries, or parallel
  page fetching.
- Change the existing 5-minute cache lifetime.

## Decisions

### Sequential cursor traversal per endpoint

Each endpoint is requested sequentially because its next cursor is only known
after parsing the preceding response. Usage requests 31 daily buckets per page;
Costs requests 180. Every request repeats the originally captured period and
endpoint-specific filters, adding only the returned page cursor.

Splitting dates into fixed 31-day windows was rejected: cursor pagination is the
API's continuation contract and avoids client-side boundary duplication or gaps.
Parallel requests were rejected because cursors are dependent and the current
desktop refresh cadence does not need extra concurrency.

### Parse and accumulate only complete page sequences

A page helper parses `data`, `has_more`, and `next_page`, tracking used cursors.
It fails if a continuation is missing or repeats. Endpoint aggregates remain
local until both page sequences finish, so the existing cache receives only a
complete `OpenAiUsage` value.

### Keep endpoint aggregation separate

Usage aggregation keeps model token totals; Costs aggregation owns daily and
monthly amounts plus the cross-page currency invariant. This prevents a generic
JSON helper from obscuring endpoint-specific validation and accounting rules.

## Risks / Trade-offs

- [Long reporting periods require more Usage requests] → preserve the current
  monthly view and use the largest supported page size for each endpoint.
- [Malformed cursors could loop indefinitely] → reject missing or repeated
  cursors before issuing another request.
- [One endpoint succeeds while the other fails] → return an error so the cache
  retains only the previous complete snapshot.

## Migration Plan

1. Add page traversal and pure aggregation helpers with regression tests.
2. Route the existing request function through those helpers.
3. Run formatting, Rust tests, and the frontend build.

Rollback is a single-code-path reversion; no stored data or IPC contract changes
are introduced.
