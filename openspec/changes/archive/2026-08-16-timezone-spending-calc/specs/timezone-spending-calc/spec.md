# timezone-spending-calc Specification

## Purpose

사용자의 시스템 로컬 시간대를 기준으로 OpenAI API 사용량과 비용(Today's spending / Current month cost)을 정확히 산출하여 시차로 인한 지출액 오집계를 방지합니다.

## ADDED Requirements

### Requirement: 시스템 로컬 시간대 기준 오늘 시작 시각 계산
오늘 지출액(`today_usage`) 산출 시 UTC 00:00:00이 아닌 사용자 시스템 로컬 시간대 기준 오늘 00:00:00(MIDNIGHT)에 해당하는 타임스탬프를 시작 기준으로 사용해야 합니다.

#### Scenario: KST 오프셋 환경에서 자정 직후 집계
- **WHEN** 한국 표준시(UTC+9) 기준 2026-08-13 01:00:00에 API 사용량을 조회할 때
- **THEN** 오늘 시작 기준 시각(`today_start`)은 KST 2026-08-13 00:00:00 (UTC 2026-08-12 15:00:00) 타임스탬프여야 한다.
- **AND** 2026-08-13 KST에 발생한 지출액이 `today_usage`에 올바르게 합산되어야 한다.

### Requirement: 시스템 로컬 시간대 기준 당월 시작 시각 계산
이번 달 지출액(`total_billed`) 및 토큰 사용량 산출 시 로컬 시간대 기준 당월 1일 00:00:00 타임스탬프부터 데이터를 요청해야 합니다.

#### Scenario: 월초 타임존 오프셋 적용
- **WHEN** 사용자의 로컬 시간대 1일 00:30:00에 사용량을 조회할 때
- **THEN** OpenAI API 조회 시작 범위(`period_start`)는 로컬 시간대 기준 해당 월 1일 00:00:00이어야 한다.

### Requirement: 로컬 타임존 획득 실패 시 UTC 폴백
시스템 로컬 오프셋 획득에 실패하거나 멀티스레드 제약으로 로컬 오프셋을 읽을 수 없는 경우 안전하게 UTC 기준으로 동작해야 합니다.

#### Scenario: 로컬 오프셋 탐색 실패
- **WHEN** OS 환경 설정 문제로 로컬 오프셋 조회가 Err를 반환할 때
- **THEN** 에러로 멈추지 않고 UTC 00:00:00 기준으로 계산을 수행하며 애플리케이션이 정상 동작해야 한다.
