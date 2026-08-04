## Why

현재 개발 환경에서는 Windows 전용 실행 경로인 WebView, 시스템 트레이, 위젯 창 동작과 Codex sidecar 기동을 실제로 검증할 수 없다. 별도의 Windows x64 PC에서 개발 도구 설치나 내부 구조 이해 없이 반복 가능한 테스트를 수행하고, 실패를 재현 가능한 정보로 보고할 수 있어야 한다.

## What Changes

- Windows x64 테스트 PC에서 실행 가능한 테스트 빌드를 준비하는 절차와 검증 기준을 제공한다.
- 앱이 Windows 런타임, 번들된 Codex sidecar, 설정 저장소의 상태를 확인할 수 있는 진단 정보를 제공한다.
- 외부 API 자격 증명 없이 대시보드·위젯·트레이 흐름을 확인할 수 있도록 예측 가능한 테스트 데이터를 제공한다.
- Windows 네이티브 환경에서 수행할 자동 smoke test 및 수동 수락 테스트 체크리스트를 정의한다.
- 공개 배포, 코드 서명, 자동 업데이트, macOS/Linux 지원은 이 변경의 범위에서 제외한다.

## Capabilities

### New Capabilities

- `windows-x64-test-support`: Windows x64에서 TokenGlass 테스트 빌드를 실행하고, 진단과 샘플 데이터를 이용해 핵심 동작을 검증하는 기능.

### Modified Capabilities

- 없음.

## Impact

- `package.json`, Tauri 설정 및 Windows 전용 스크립트에 테스트 빌드·검증 진입점을 추가할 수 있다.
- Rust 백엔드와 React UI에 테스트 데이터/진단 상태를 노출하는 경로가 추가될 수 있다.
- Windows 테스트 가이드와 자동 검증 환경이 추가된다.
- 기존 OpenAI API 및 ChatGPT OAuth의 실제 운영 흐름은 유지하며, 테스트 자격 증명이나 OAuth 토큰을 저장소·로그·테스트 결과물에 기록하지 않는다.
