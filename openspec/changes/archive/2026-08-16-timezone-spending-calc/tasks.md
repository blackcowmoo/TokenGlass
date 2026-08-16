# Tasks: Local Timezone Spending Calculation

- [x] Cargo.toml 및 Rust dependencies 점검 <!-- id: 0 -->
  - [x] `src-tauri/Cargo.toml`의 `time` 크레이트에 `local-offset` 피처 설정이 포함되어 있는지 확인 및 조정 <!-- id: 1 -->

- [x] Backend 타임존 계산 로직 구현 (`src-tauri/src/lib.rs`) <!-- id: 2 -->
  - [x] 로컬 타임존 오프셋을 추출하고 자정(00:00:00) 타임스탬프를 안전하게 산출하는 헬퍼 함수 구현 <!-- id: 3 -->
  - [x] `fetch_openai_usage` 함수 내 `period_start` 및 `today_start` 계산 로직을 로컬 타임존 기반으로 교체 <!-- id: 4 -->
  - [x] Costs API 버킷의 `start_time` 파싱 시 로컬 오늘 기준 필터링 정확도 개선 및 UTC 폴백 예외 처리 구현 <!-- id: 5 -->

- [x] Rust 단위 테스트 추가 (`src-tauri/src/lib.rs`) <!-- id: 6 -->
  - [x] 로컬 오프셋 변환 및 자정 시각 타임스탬프 산출에 대한 `#[test]` 단위 테스트 구현 및 검증 <!-- id: 7 -->

- [x] Frontend UI 텍스트 및 상태 표시 정비 (`src/App.tsx`) <!-- id: 8 -->
  - [x] 로컬 시간대 기준 집계 안내 툴팁 또는 캡션 텍스트 적용 및 UI 연동 확인 <!-- id: 9 -->

- [x] 빌드 및 실행 수동 검증 <!-- id: 10 -->
  - [x] `pnpm tauri dev` 및 `cargo test` 실행하여 단위 테스트 및 TypeScript 검증 완료 <!-- id: 11 -->

