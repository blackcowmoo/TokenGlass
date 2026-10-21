## Why

OpenAI Costs API 비용은 원 통화 금액인데 화면이 달러 기호를 하드코딩해 통화의 출처와 원화 환산 기준을 사용자가 알거나 조정할 수 없다. 외부 환율 서비스에 의존하지 않고 사용자가 지정한 환율로 KRW 참고 금액을 확인할 수 있어야 한다.

## What Changes

- 비용 응답의 원본 통화 코드를 보존하고 USD 비용을 명시적으로 표시한다.
- 설정 화면에 사용자가 저장할 USD→KRW 수동 환율을 추가한다.
- 메인 대시보드와 데스크톱 위젯에 원본 USD 및 수동 환율 기준 KRW 참고 환산액을 동일하게 표시한다.
- 입력값이 유효하지 않으면 마지막으로 저장된 유효 환율 또는 기본 환율을 계속 사용한다.

## Capabilities

### New Capabilities

- `manual-krw-exchange-rate`: OpenAI API 비용의 USD 원본 금액과 사용자 설정 KRW 환산 표시 및 설정 저장 동작을 정의한다.

### Modified Capabilities

- 없음.

## Impact

- `src/usage.ts`의 비용 데이터·금액 포매팅 계약
- `src/App.tsx`, `src/Widget.tsx`의 설정 로드 및 비용 표시
- Tauri Store의 `settings.json` 수동 환율 설정 키
- `src-tauri/src/lib.rs`의 Costs API 통화 코드 파싱 및 응답 계약
