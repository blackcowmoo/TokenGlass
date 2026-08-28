## 1. Shared Usage Contract

- [x] 1.1 Add the Rust async synchronization and API-key fingerprint dependencies required by the shared cache design.
- [x] 1.2 Move the OpenAI Usage/Costs network request and aggregation into a function that can be called independently of cache policy.
- [x] 1.3 Define the Rust usage snapshot response with usage data, `fetchedAt`, response source, stale state, and optional refresh error.
- [x] 1.4 Add shared TypeScript types for usage data and the usage snapshot, then replace the duplicate component-local usage types.

## 2. Backend Cache and Refresh Coordination

- [x] 2.1 Add Tauri-managed OpenAI usage state containing the in-memory cache entry, API-key fingerprint, generation number, and refresh gate.
- [x] 2.2 Implement the 5-minute TTL lookup and API-key isolation rules without storing or exposing the raw API key in cache metadata.
- [x] 2.3 Implement refresh-gate coordination and generation rechecks so concurrent normal and force-refresh requests share one completed network refresh.
- [x] 2.4 Update `fetch_openai_usage` to accept `forceRefresh`, return fresh or cached snapshots, and return a stale snapshot with the last successful data when a later refresh fails.
- [x] 2.5 Register the OpenAI usage state during Tauri startup and confirm no usage snapshot is written to Tauri Store or another persistent location.

## 3. Main Dashboard Refresh UX

- [x] 3.1 Update the initial dashboard load to consume the snapshot response and display the backend-provided last successful time.
- [x] 3.2 Add a 5-minute automatic refresh lifecycle with cleanup and trigger a normal refresh when the hidden main window becomes active again.
- [x] 3.3 Make the toolbar refresh and API-key save actions request a force refresh while preventing overlapping UI actions.
- [x] 3.4 Preserve displayed usage on stale or failed follow-up refreshes and show loading, stale-error, and last-success information without treating cache reads as new successes.

## 4. Desktop Widget Integration

- [x] 4.1 Update the widget startup and 5-minute timer requests to consume the shared snapshot contract without forcing network refreshes.
- [x] 4.2 Keep the last successful widget value visible when a follow-up refresh returns stale data or fails, while reporting refresh errors without exposing credentials.

## 5. Verification

- [x] 5.1 Add Rust tests for TTL boundaries, API-key fingerprint mismatch, force-refresh decisions, cache generation changes, and stale fallback behavior.
- [x] 5.2 Add a concurrency-focused Rust test proving simultaneous requests for one API key result in one refresh generation.
- [x] 5.3 Run Rust tests and the TypeScript/Vite production build, fixing all cache contract and lifecycle regressions.
- [x] 5.4 Verify app startup, manual refresh, five-minute refresh, hidden-window reactivation, widget coexistence, API-key replacement, and refresh-failure states in the Tauri app without recording credentials.
