## Why

현재 `lib.rs` 백엔드에서 `OffsetDateTime::from_unix_timestamp(now)`를 사용하여 UTC 자정 기준(`today_start`)으로 당일 지출액(Today's API spending)을 집계하고 있습니다.
이로 인해 한국 표준시(KST, UTC+9) 등 로컬 시간대 사용자가 오전 00:00 ~ 09:00 사이에 앱을 이용할 때, 로컬 '오늘'의 시작 시점과 UTC '오늘'의 시작 시점이 일치하지 않아 오늘 자금 지출액이 0달러로 표기되거나 어제 금액으로 잘못 집계되는 현상이 발생합니다.

## What Changes

- Rust 백엔드(`lib.rs`)에서 시스템의 로컬 오프셋(`UtcOffset::current_local_offset()`) 또는 타임존 정보를 추출하여 **로컬 자정(00:00:00)** 및 **로컬 1일 00:00:00**을 기준으로 `today_start` 및 `period_start` UNIX 타임스탬프를 계산합니다.
- OpenAI Costs API 버킷 조회 결과에서 `start_time`을 로컬 타임존 시각으로 오프셋 변환한 뒤 오늘 집계에 포함 여부를 정확히 판단합니다.
- 프론트엔드(`App.tsx`) UI 텍스트에 "Today's API spending (Local)" 및 "Current Month (Local)" 표기 방식을 반영하여 타임존 집계 기준을 직관적으로 제공합니다.
- 시간대 변환 실패 시 안전하게 UTC로 롤백되는 폴백(Fallback) 방어 로직을 추가합니다.

## Capabilities

### New Capabilities

- 없음.

### Modified Capabilities

- `openai-usage-tracking`: UTC 대신 사용자의 시스템 로컬 시간대 자정을 기준으로 '오늘'과 '이번 달'의 토큰/비용 집계 범위를 산출하는 방식으로 개선.

## Impact

- `src-tauri/src/lib.rs`: `fetch_openai_usage` 내부의 타임스탬프 산출 로직 및 비용 버킷 필터링 구문 변경.
- `src-tauri/Cargo.toml`: `time` 크레이트의 `local-offset` 기능 플래그 활성화 확인 또는 필요한 로컬 오프셋 도구 확인.
- `src/App.tsx`: 상태 설명 및 헤더 캡션 문자열 일부 수정.
- 기존 OpenAI API 키 저장 및 연동 구조에는 영향을 주지 않으며, 네트워크 데이터 형식 변경 없이 서버에서 전달받은 타임스탬프 해석 로직만 수정됩니다.
