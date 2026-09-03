## Purpose

Intel 기반 macOS 사용자가 별도 Codex CLI 설치 없이 번들 Codex App Server를 실행해 ChatGPT 구독 사용량 기능을 사용할 수 있게 한다.

## ADDED Requirements

### Requirement: Intel macOS Codex sidecar 준비

시스템은 Intel 기반 macOS에서 빌드 또는 개발 실행 전에 `x86_64-apple-darwin` Codex App Server sidecar를 자동으로 준비해야 한다 (MUST). 준비된 sidecar는 Tauri 번들러가 포함하고 실행할 수 있는 이름과 실행 권한을 가져야 한다 (MUST).

#### Scenario: Intel macOS의 sidecar 준비

- **WHEN** Intel 기반 macOS에서 sidecar 준비 명령을 실행할 때
- **THEN** 시스템은 공식 Codex 설치 프로그램으로 Intel macOS용 실행 파일을 준비하고 Tauri가 발견할 수 있는 `codex-x86_64-apple-darwin` 파일을 생성해야 한다

#### Scenario: 준비된 sidecar 재사용

- **WHEN** 실행 가능한 `codex-x86_64-apple-darwin` sidecar가 이미 있을 때
- **THEN** 시스템은 설치 프로그램을 다시 실행하지 않고 기존 sidecar를 재사용해야 한다

### Requirement: Intel macOS 지속적 통합 검증

시스템은 Intel macOS CI 환경에서 sidecar 준비과 Tauri 앱 번들 생성을 검증해야 한다 (MUST). 검증은 Intel 아키텍처에서 실행되고, 실행 가능한 `codex-x86_64-apple-darwin` sidecar가 준비된 것을 확인해야 한다 (MUST).

#### Scenario: Intel macOS CI 검증 성공

- **WHEN** 지원 대상 브랜치로 push하거나 pull request가 열릴 때
- **THEN** CI는 Intel macOS 환경에서 sidecar를 준비하고, 실행 권한 및 target 파일명을 확인한 뒤 Tauri 앱 번들을 생성해야 한다

### Requirement: Intel macOS 지원 범위 안내

시스템은 문서에서 Apple Silicon 및 Intel 기반 macOS가 자동 sidecar 준비 대상임을 정확히 안내해야 한다 (MUST).

#### Scenario: 개발자 지원 범위 확인

- **WHEN** 개발자가 프로젝트의 sidecar 준비 지원 범위를 확인할 때
- **THEN** 문서는 Intel 기반 macOS가 자동 준비 대상임을 명시해야 한다
