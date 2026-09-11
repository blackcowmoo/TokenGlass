## Why

TokenGlass는 Usage/Costs 권한을 가진 OpenAI 조직 관리자 API 키를 Tauri Store의
`settings.json`에 JSON 값으로 저장하고 있어, 같은 사용자 권한으로 실행되는 프로세스가
원본 키를 읽을 수 있다. GitHub 이슈 #7을 해결하기 위해 지원 OS의 네이티브 보안 저장소로
키를 이전하고 키가 일반 설정, 진단 정보, 오류 또는 캐시에 노출되지 않는 동작을 정의한다.

## What Changes

- macOS Keychain, Windows Credential Manager, Linux Secret Service를 하나의 Rust 보안 자격
  증명 저장소 경계로 사용한다.
- API 키 저장 여부만 프런트엔드에 공개하고, 사용량 조회 시 원본 키가 WebView 상태나 Tauri
  명령 인자로 왕복하지 않도록 OpenAI 명령 경계를 변경한다.
- 기존 `settings.json`의 `tokenglass_openai_admin_key` 값을 보안 저장소로 한 번만 이전하고,
  안전한 기록을 확인한 후 원본 값을 삭제한다.
- 키 저장, 갱신, 삭제 및 보안 저장소 조회 실패에 대한 사용자 동작과 오류 처리를 정의하며,
  보안 저장소 사용 불가 시 평문 영속 저장소로 자동 폴백하지 않는다.
- 진단 정보, 사용자 화면, 오류, 로그 및 사용량 캐시에 원본 API 키가 포함되지 않음을 자동 및
  수동 검증한다.
- 일반 설정 Store는 수동 환율 같은 비밀이 아닌 설정에만 계속 사용한다.

## Capabilities

### New Capabilities

- `secure-openai-key-storage`: OpenAI 조직 관리자 API 키의 OS 네이티브 보안 저장, 상태 조회,
  갱신, 삭제, 기존 Store 마이그레이션 및 비밀 비노출 동작을 정의한다.

### Modified Capabilities

없음.

## Impact

- `src-tauri/src/`에 OS 자격 증명 저장소를 소유하는 Rust 모듈과 Tauri 명령이 추가되고,
  OpenAI 사용량 명령은 저장된 키를 백엔드에서 조회하도록 변경된다.
- `src/App.tsx`와 `src/Widget.tsx`는 Store에서 원본 키를 읽지 않고 보안 키 상태 및 백엔드
  사용량 명령만 사용하도록 변경된다.
- Rust 보안 저장소 의존성과 지원 OS별 빌드 설정이 추가되며 Linux 배포 환경은 Secret
  Service 및 사용자 D-Bus 세션의 가용성을 고려해야 한다.
- `settings.json`의 기존 키 삭제, 캐시 무효화, 오류 분류, 진단 마스킹 및 Windows/macOS/Linux
  수동 검증 범위가 추가된다.
