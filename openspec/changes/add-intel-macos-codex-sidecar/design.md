## Context

현재 Node 준비 래퍼와 POSIX 설치 스크립트는 Apple Silicon macOS만 `aarch64-apple-darwin`으로 매핑한다. Tauri의 `externalBin` 설정은 `binaries/codex` 접두사에 현재 플랫폼 target triple을 붙인 sidecar 파일을 포함한다.

## Goals / Non-Goals

**Goals:**

- Intel macOS의 Node 런타임 및 셸 아키텍처를 모두 `x86_64-apple-darwin`으로 매핑한다.
- 설치 프로그램이 반환한 실행 파일을 해당 target 파일명으로 정규화한다.
- 매핑이 regression 없이 유지되도록 자동 검증한다.

**Non-Goals:**

- Linux 또는 Windows 지원 범위 변경
- Codex 설치 프로그램의 릴리스 선택·다운로드 방식 변경
- cross-compilation 또는 Intel macOS 환경에서의 실제 Tauri 번들 생성

## Decisions

### 기존 sidecar 파일명 규칙을 유지한다

Tauri가 `externalBin` 접두사와 플랫폼 target triple을 결합해 sidecar를 찾으므로, 설치기가 노출하는 일반 `codex` 실행 파일을 `codex-x86_64-apple-darwin`으로 복사한다. 설치 프로그램의 내부 배포 파일명을 직접 의존하는 방식보다 기존 Apple Silicon/Linux 흐름과 일관되고 설치기 레이아웃 변경에 강하다.

### Node와 셸의 플랫폼 판정을 같은 지원 집합으로 확장한다

Node 래퍼는 `darwin/x64`, POSIX 스크립트는 `Darwin/x86_64`를 각각 Intel macOS로 처리한다. 양쪽 모두를 수정해야 개발 실행과 빌드 실행의 preflight/설치 단계가 불일치하지 않는다.

### 플랫폼 판정은 격리 가능한 Node 모듈로 검증한다

target 선택 로직을 순수 모듈로 분리하고 Node 내장 테스트로 지원/미지원 조합을 검사한다. 별도 Intel macOS 호스트 없이도 매핑 및 기존 지원 회귀를 확인할 수 있다.

## Risks / Trade-offs

- [공식 설치 프로그램이 Intel macOS 배포물을 중단할 수 있음] → 설치 실패를 현재와 같이 명확히 전파하고, 준비 시점에만 외부 의존성을 사용한다.
- [현재 호스트가 Apple Silicon이라 실제 Intel 바이너리 실행은 확인하지 못함] → target 선택·파일명 정규화는 자동 테스트로 검증하고 Intel macOS에서 후속 smoke test를 수행한다.

## Migration Plan

1. Intel macOS target 매핑과 설치 후보를 추가한다.
2. target 선택 테스트와 문서를 추가한다.
3. 현재 macOS에서 기존 Apple Silicon 준비·빌드 경로를 검증한다.
4. Intel macOS 호스트에서 `pnpm prepare:sidecar` 및 Tauri 패키징 smoke test를 수행한다.
