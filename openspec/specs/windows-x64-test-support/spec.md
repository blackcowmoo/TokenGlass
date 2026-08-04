# windows-x64-test-support Specification

## Purpose

Windows x64 테스트 PC에서 외부 자격 증명이나 개발 도구 없이 TokenGlass의 핵심 흐름을 반복 검증하고, 실패를 안전하게 보고할 수 있게 한다.

## Requirements

### Requirement: Windows x64 테스트 실행물
시스템은 Windows x64에서 실행할 수 있는 테스트 전용 실행물을 제공해야 한다 (MUST). 테스트 실행물에는 TokenGlass가 Codex App Server를 시작하는 데 필요한 Windows x64 sidecar가 포함되어야 하며, 테스트 PC 사용자는 Node.js, pnpm, Rust 또는 Codex CLI를 별도로 설치하지 않고 실행할 수 있어야 한다.

#### Scenario: 개발 도구가 없는 테스트 PC에서 시작
- **WHEN** 테스트 담당자가 지원되는 Windows x64 PC에서 테스트 실행물을 시작한다
- **THEN** TokenGlass가 개발 도구 또는 별도 Codex CLI 설치를 요구하지 않고 시작되어야 한다

#### Scenario: 누락된 sidecar 감지
- **WHEN** 테스트 실행물에서 필요한 Windows x64 sidecar를 찾을 수 없거나 실행할 수 없다
- **THEN** 시스템은 기능을 사용할 수 없음을 명확히 알리고 민감 정보를 포함하지 않는 진단 상태를 제공해야 한다

### Requirement: 자격 증명 없는 테스트 모드
시스템은 테스트 전용 실행물에서 외부 API 요청, 관리자 API 키 또는 ChatGPT OAuth 로그인 없이 대시보드, 트레이 및 위젯의 핵심 화면 흐름을 확인할 수 있는 예측 가능한 샘플 데이터를 제공해야 한다 (MUST). 시스템은 샘플 데이터가 실제 사용량이 아님을 명확히 표시해야 한다.

#### Scenario: 테스트 모드 대시보드 확인
- **WHEN** 테스트 담당자가 테스트 전용 실행물을 시작한다
- **THEN** 대시보드는 샘플 사용량을 표시하고 테스트 모드임을 식별 가능하게 표시해야 한다

#### Scenario: 테스트 모드 위젯 확인
- **WHEN** 테스트 담당자가 트레이에서 위젯을 표시한다
- **THEN** 위젯은 샘플 오늘 사용량을 표시하고 실제 관리자 API 키를 요구하지 않아야 한다

### Requirement: Windows 핵심 흐름 수락 검증
시스템은 Windows x64 테스트 담당자가 반복 실행할 수 있는 수동 수락 테스트 절차를 제공해야 한다 (MUST). 절차는 앱 시작, 트레이에서 대시보드 표시 및 숨김, 위젯 표시 및 숨김, 설정 상태 복원, 정상·실패한 API 사용량 요청, ChatGPT OAuth 시작, 다중 모니터 및 화면 배율 확인을 포함해야 한다.

#### Scenario: 체크리스트에 따른 핵심 흐름 확인
- **WHEN** 테스트 담당자가 제공된 Windows 수락 테스트 절차를 수행한다
- **THEN** 각 핵심 흐름의 기대 결과와 성공 또는 실패 기록 방법을 확인할 수 있어야 한다

### Requirement: 안전한 진단 정보
시스템은 테스트 담당자가 앱 버전, Windows 버전, 실행 모드, sidecar 준비 및 기동 상태, 저장소 접근 상태, 최근 오류 메시지를 확인하고 전달할 수 있는 진단 정보를 제공해야 한다 (MUST). 진단 정보에는 관리자 API 키, OAuth 토큰, 인증 헤더 또는 사용자가 입력한 원문을 포함해서는 안 된다.

#### Scenario: 문제 재현 정보 전달
- **WHEN** 테스트 중 오류가 발생하고 담당자가 진단 정보를 수집한다
- **THEN** 재현에 필요한 비밀이 아닌 상태와 오류 정보를 전달할 수 있어야 한다

#### Scenario: 자격 증명 보호
- **WHEN** 테스트 담당자가 관리자 API 키를 입력한 뒤 진단 정보를 수집한다
- **THEN** 진단 정보와 테스트 결과물에 해당 키 또는 OAuth 토큰이 포함되어서는 안 된다

### Requirement: Windows x64 반복 가능 검증
시스템은 Windows x64 환경에서 테스트 실행물 생성 및 기동을 확인하는 자동 smoke test 절차를 제공해야 한다 (MUST). 검증 절차는 생성된 실행물의 Windows x64 sidecar 존재 여부와 앱 기동 가능 여부를 확인해야 하며, 실제 OpenAI 또는 ChatGPT 계정에 의존해서는 안 된다.

#### Scenario: Windows smoke test 성공
- **WHEN** 지원되는 Windows x64 환경에서 자동 smoke test를 실행한다
- **THEN** 테스트는 테스트 실행물과 Windows x64 sidecar를 확인하고 외부 계정 없이 기동 성공을 보고해야 한다

#### Scenario: smoke test 실패 보고
- **WHEN** 테스트 실행물 생성, sidecar 확인 또는 앱 기동 중 하나가 실패한다
- **THEN** 테스트는 실패 단계를 명확히 표시하고 비밀을 출력하지 않아야 한다
