## Context

현재 JavaScript/TypeScript/CSS/JSON 포맷터, 편집기 규칙, 루트 포맷 스크립트, CI 포맷 검사가 없다. Rust에는 표준 `cargo fmt`가 제공되지만 프로젝트 명령이나 GitHub Actions에서 실행되지 않는다.

## Goals / Non-Goals

**Goals:**

- 프런트엔드와 설정 파일에 단일 포맷터를 적용한다.
- Rust와 프런트엔드 형식을 한 번에 정리·검사할 수 있는 명령을 제공한다.
- pull request와 기본 브랜치 변경에서 형식 위반을 CI가 차단한다.

**Non-Goals:**

- 코드 품질 규칙 전체를 강제하는 ESLint 또는 Clippy 정책
- Git hooks나 자동 커밋 포맷
- 기존 빌드·테스트 CI를 새로 설계하는 작업

## Decisions

### Biome과 cargo fmt를 분리해 사용한다

Biome을 TypeScript, TSX, CSS, JSON, Markdown 및 YAML의 포맷터로 사용하고, Rust는 Rust 표준 도구인 `cargo fmt`를 유지한다. Biome은 단일 설정과 빠른 실행을 제공하며 Prettier와 ESLint를 별도로 도입할 필요가 없다.

`format`은 두 도구를 수정 모드로 실행하고, `format:check`는 파일을 수정하지 않고 둘 다 검사한다. 설정 파일은 포맷 범위에서 생성물·의존성·Rust 빌드 산출물을 제외한다.

### CI는 포맷 전용 작업을 제공한다

GitHub Actions 워크플로는 pull request와 기본 브랜치 push에서 의존성을 재현 가능하게 설치하고 `pnpm format:check`를 실행한다. 포맷 실패는 작업 실패로 처리하며, 포맷 외 빌드·테스트 실패와 분리해 원인을 빠르게 알 수 있게 한다.

## Risks / Trade-offs

- [초기 일괄 포맷으로 큰 diff가 생길 수 있음] → 포맷 설정과 소스 정리를 같은 변경에 포함하고 이후 diff를 안정화한다.
- [Biome이 지원하지 않는 파일이 있을 수 있음] → 지원 범위를 명시하고 Rust는 cargo fmt로 별도 처리한다.
- [개발 환경의 Node/pnpm 차이] → lockfile 기반 설치와 package script를 CI와 로컬에서 공통으로 사용한다.

## Migration Plan

1. Biome 및 EditorConfig 설정과 package scripts를 추가한다.
2. 전체 포맷을 실행해 현재 소스 기준선을 맞춘다.
3. GitHub Actions 포맷 검사 워크플로를 추가한다.
4. 로컬 검사와 CI 트리거를 확인한다.

문제가 생기면 워크플로와 스크립트를 제거해 기존 개발 흐름으로 되돌릴 수 있으며, 소스 포맷 변경은 동일 규칙으로 다시 적용 가능하다.
