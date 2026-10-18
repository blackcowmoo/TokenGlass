## 1. 보안 저장소 기반

- [ ] 1.1 `keyring` 3.6 계열을 macOS Keychain, Windows Credential Manager, Linux Secret Service 및 RustCrypto 기능으로 추가하고 host `cargo check`와 지원 target별 의존성 해석이 성공하는지 확인한다.
- [x] 1.2 `src-tauri/src/openai/credentials.rs`에 고정 service/account 식별자, get/set/delete 저장소 경계와 비밀 없는 공개 오류 분류를 구현하고 메모리 저장소 단위 테스트로 저장·갱신·조회·멱등 삭제를 검증한다.
- [ ] 1.3 동기 OS 자격 증명 호출을 Tauri 비동기 런타임의 blocking 작업으로 격리하고 성공·거부·잠김·저장소 부재 결과가 예상한 공개 오류로 변환되는지 단위 테스트한다.

## 2. 백엔드 명령과 캐시 수명 주기

- [x] 2.1 `OpenAiUsageState`에 명시적 캐시 무효화 동작을 추가하고 키 저장·삭제 성공 시 캐시가 제거되며 저장 실패 시 기존 캐시가 유지되는지 Rust 테스트로 검증한다.
- [ ] 2.2 `get_openai_key_status`, `save_openai_admin_key`, `delete_openai_admin_key` Tauri 명령을 구현·등록하고 응답이 설정 여부와 비밀 없는 상태만 직렬화하는지 명령 경계 테스트로 검증한다.
- [ ] 2.3 `fetch_openai_usage`에서 `adminKey` 입력을 제거하고 보안 저장소에서 키를 읽도록 변경하며 미설정·저장소 오류·정상 조회 경로와 기존 캐시 격리 테스트가 통과하는지 확인한다.

## 3. 기존 Store 마이그레이션

- [x] 3.1 Rust 측 legacy Store 접근과 직렬화된 `ensure_migrated` 초기화를 구현하고 보안 저장소가 비어 있을 때 기록·read-back 확인 후에만 `tokenglass_openai_admin_key`를 삭제하는지 테스트한다.
- [x] 3.2 마이그레이션의 기록 실패, read-back 불일치, Store 정리 실패, 양쪽 값 존재 및 둘 이상의 WebView 동시 요청을 테스트하고 데이터 손실 없이 보안 저장소 우선·정리 재시도 규칙이 유지되는지 확인한다.
- [x] 3.3 키 상태 조회와 사용량 조회가 공통 초기화 경계를 통과하도록 앱 상태를 등록하고 앱 재시작 시 마이그레이션이 멱등인지 통합 테스트 또는 실제 Store fixture로 검증한다.

## 4. 메인 화면과 위젯 전환

- [ ] 4.1 `src/App.tsx`에서 Store 기반 키 로드·저장을 제거하고 저장 여부 표시, 빈 교체 입력, 명시적 삭제와 새 백엔드 명령을 연결한 뒤 프런트엔드 빌드와 설정 화면 수동 동작을 확인한다.
- [ ] 4.2 `src/Widget.tsx`에서 Store의 원본 키 조회와 `adminKey` 명령 인자를 제거하고 저장된 백엔드 키로 초기·주기 조회가 동작하는지 프런트엔드 빌드와 위젯 수동 실행으로 확인한다.
- [x] 4.3 소스와 생성되지 않은 설정 산출물에서 `tokenglass_openai_admin_key` 사용이 Rust 마이그레이션 경계에만 남고 원본 키를 반환하는 IPC 계약이 없는지 저장소 검색으로 검증한다.

## 5. 비밀 비노출과 운영 문서

- [ ] 5.1 실제 런타임 오류·진단 생성 경로에 canary API 키를 주입하는 회귀 테스트를 추가하고 화면 문자열, 복사 진단, 로그 및 캐시 직렬화 결과에 canary 값이 없음을 검증한다.
- [x] 5.2 macOS·Windows·Linux의 기존 Store 위치와 위험 수준, 네이티브 저장소 선택 근거, Linux Secret Service/D-Bus 부재 대응, 사용자 키 삭제 및 다운그레이드 시 재입력 정책을 문서화하고 문서의 명령과 경로를 검토한다.
- [ ] 5.3 macOS에서 legacy 키 마이그레이션, Keychain 저장·갱신·삭제, `settings.json` 원본 제거와 앱 재시작 조회를 canary 키로 수동 검증하고 비밀이 없는 결과만 기록한다.
- [ ] 5.4 Windows x64와 Linux x64에서 각 네이티브 저장소의 저장·갱신·삭제·재시작을 수동 검증하고 Linux에서는 Secret Service 정상·잠김·부재 시나리오가 평문 폴백 없이 동작하는지 확인한다.

## 6. 품질 게이트

- [x] 6.1 변경 파일을 포맷하고 `cargo test --manifest-path src-tauri/Cargo.toml`, `pnpm build`, 관련 진단 테스트가 모두 통과하는지 확인한다.
- [ ] 6.2 Intel/Apple Silicon macOS, Windows x64 및 Linux x64의 기존 빌드·패키징 경로에서 새 플랫폼 의존성이 해석되고 결과물이 실행되는지 CI 또는 대상 환경 빌드 증적으로 확인한다.
- [ ] 6.3 OpenSpec 변경을 strict 검증하고 모든 완료 조건과 검증 증적이 GitHub 이슈 #7의 네 가지 완료 조건을 충족하는지 최종 대조한다.
