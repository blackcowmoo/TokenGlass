## Why

Intel 기반 macOS 사용자는 현재 Codex App Server 사이드카를 준비할 수 없어 ChatGPT 구독 사용량 기능을 이용할 수 없다. 공식 Codex 배포물은 Intel macOS target을 제공하므로, 지원 가능한 플랫폼을 애플리케이션의 준비 흐름에 반영한다.

## What Changes

- Intel macOS에서 `x86_64-apple-darwin` Codex 사이드카를 자동으로 준비한다.
- 설치 결과를 Tauri가 인식하는 Intel macOS sidecar 파일명으로 정규화한다.
- Intel macOS 지원 범위를 README에 반영한다.
- 플랫폼 target 선택을 자동 검증해 지원 매핑이 변경으로 깨지지 않게 한다.
- Intel macOS GitHub Actions에서 sidecar 준비와 Tauri 앱 번들을 검증한다.

## Capabilities

### New Capabilities
- `intel-macos-codex-sidecar`: Intel macOS용 번들 Codex App Server 사이드카 준비 및 검증.

### Modified Capabilities

- 없음.

## Impact

- `scripts/prepare-codex-sidecar.mjs`
- `scripts/prepare-codex-sidecar.sh`
- `package.json` 및 sidecar 준비 테스트
- `README.md`
- `.github/workflows/`
