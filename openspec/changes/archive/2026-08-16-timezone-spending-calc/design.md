# Design Document: Local Timezone Spending Calculation

## Overview

TokenGlass는 사용자가 실시간으로 OpenAI 토큰 비용을 파악할 수 있도록 위젯과 대시보드를 제공합니다.
기존 구현에서는 Rust `time` 크레이트로 UTC 자정을 계산했기 때문에, 한국(KST, UTC+9) 등 UTC와 시차가 존재하는 지역에서는 오전 시간대에 '오늘의 지출액'이 어제 집계에 머무르거나 당일 지출이 누락되는 문제가 있었습니다.
본 설계는 사용자 OS의 로컬 타임존 오프셋을 추출하여 '오늘 자정(00:00:00)'과 '당월 1일 자정(00:00:00)'의 Unix Timestamp를 정확하게 계산하는 구조를 정의합니다.

## Architecture & Data Flow

```
┌──────────────────────────────────────────────────────────────────────────┐
│                            Timezone Calculation                          │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  System Clock (UNIX Timestamp: 1786550400)                               │
│        │                                                                 │
│        ▼                                                                 │
│  UtcOffset::current_local_offset() ──(Fallback on Err)──▶ UtcOffset::UTC │
│        │                                                                 │
│        ▼                                                                 │
│  OffsetDateTime::from_unix_timestamp(...) + Local Offset                 │
│        │                                                                 │
│        ├──▶ local_now.date() -> Year, Month, Day                         │
│        ├──▶ period_start: Local Month 1st 00:00:00 -> UNIX Timestamp      │
│        └──▶ today_start: Local Today 00:00:00 -> UNIX Timestamp          │
│                                                                          │
│  OpenAI Cost Bucketing Filter                                            │
│        │                                                                 │
│        ▼                                                                 │
│  bucket.start_time (UTC Timestamp) >= today_start ?                      │
│        ├── Yes ──▶ Add amount to `today_usage`                           │
│        └── No  ──▶ Add amount to `total_billed` (Current Month) only     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

## Detailed Technical Changes

### 1. Cargo.toml `time` Crate Feature Check
Rust `time` 크레이트에서 `UtcOffset::current_local_offset()`을 사용하려면 `local-offset` 피처가 활성화되어 있어야 합니다.
`src-tauri/Cargo.toml`을 점검하고 필요시 `time = { version = "0.3", features = ["formatting", "parsing", "local-offset"] }`로 업데이트합니다.

### 2. Rust Backend (`src-tauri/src/lib.rs`)

`fetch_openai_usage` 모듈 내 시간 계산부 수정:

```rust
fn get_local_midnight_timestamps() -> (i64, i64, i64) {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let local_offset = time::UtcOffset::current_local_offset()
        .unwrap_or(time::UtcOffset::UTC);

    let local_now = time::OffsetDateTime::from_unix_timestamp(now_secs)
        .map(|dt| dt.to_offset(local_offset))
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    let period_start = time::Date::from_calendar_date(local_now.year(), local_now.month(), 1)
        .ok()
        .and_then(|d| d.with_hms(0, 0, 0).ok())
        .map(|dt| dt.assume_offset(local_offset).unix_timestamp())
        .unwrap_or(now_secs - 30 * 86400);

    let today_start = time::Date::from_calendar_date(local_now.year(), local_now.month(), local_now.day())
        .ok()
        .and_then(|d| d.with_hms(0, 0, 0).ok())
        .map(|dt| dt.assume_offset(local_offset).unix_timestamp())
        .unwrap_or(now_secs - 86400);

    (period_start, today_start, now_secs)
}
```

### 3. Cost Bucket Categorization
OpenAI Costs API 결과 파싱 시:
- `bucket.start_time` (Unix timestamp)이 `today_start` 이상이거나, 버킷 시간 범위가 로컬 자정 이후인 경우 `today_usage`에 합산합니다.

### 4. React Frontend (`src/App.tsx`)
- `Today's API spending` 항목 라벨을 유지하되, 상태 툴팁이나 설명 부분에 로컬 자정 기준 집계임을 인식할 수 있도록 직관적 텍스트 정비.

## Verification Plan

### Automated Tests
- Rust unit test (`src-tauri/src/lib.rs` 모듈 내 `#[cfg(test)]` 추가)
  - `get_local_midnight_timestamps()` 테스트: 특정 타임스탬프와 오프셋(+09:00, -05:00, +00:00) 주입 시 자정 타임스탬프 계산 검증.

### Manual Verification
- 테스트 모드(`TOKENGLASS_TEST_MODE=true`) 및 실제 API 응답 시 자정 전후 데이터 파싱 상태 확인.
