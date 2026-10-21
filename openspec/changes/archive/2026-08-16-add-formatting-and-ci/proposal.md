## Why

프로젝트에 TypeScript/React/CSS와 Rust 전체에 적용되는 일관된 포맷터, 로컬 검사 명령, CI 형식 검사가 없다. 개발자와 CI가 동일한 규칙으로 형식 문제를 빠르게 발견하고 자동 정리할 수 있어야 한다.

## What Changes

- Biome을 TypeScript, TSX, CSS, JSON 및 설정 파일의 공통 포맷터로 추가한다.
- Rust 코드에는 표준 `cargo fmt`를 사용하고, 루트 명령으로 두 포맷터를 함께 실행하거나 검사한다.
- 편집기 기본 설정을 `.editorconfig`와 Biome 설정으로 명시한다.
- GitHub Actions에 포맷 검사 작업을 추가해 포맷이 맞지 않는 변경을 검증한다.
- 기존 소스 전체를 새 규칙으로 일괄 포맷한다.

## Capabilities

### New Capabilities

없음. 이 변경은 제품 동작이 아닌 개발 도구 및 CI 정책만 추가한다.

### Modified Capabilities

없음.

## Impact

- `package.json`, Biome 및 EditorConfig 설정
- `src/`, `scripts/`, 루트 JSON/설정 파일의 형식
- `src-tauri/` Rust 형식 검사
- `.github/workflows/` CI 워크플로
