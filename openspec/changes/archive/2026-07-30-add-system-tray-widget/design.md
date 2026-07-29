## Context

Tauri 프레임워크를 기반으로 React(Vite) 프론트엔드와 Rust 백엔드가 구축되어 있으며, 내부적으로 `codex` 사이드카를 통해 OAuth 세션을 유지하고 API 사용량을 가져오는 기능이 이미 안정적으로 동작 중입니다. (proposal.md 참조)

## Goals / Non-Goals

**Goals:**
- 데스크탑에서 자연스럽게 동작하는 트레이 아이콘과 팝업 창 구현
- 투명한 항상 위 바탕화면 위젯 구현을 위한 다중 창(Multi-Window) 관리

**Non-Goals:**
- 새로운 AI 서비스(Gemini, Claude) 연동 기능은 이번 변경 범위에서 제외 (추후 별도 Change로 진행)

## Decisions

### 1. 트레이 팝업 창의 윈도우 관리 (Tauri Tray Icon API)
- **결정**: `tauri::tray::TrayIconBuilder`를 사용하여 백엔드에서 트레이 아이콘을 생성하고, 마우스 클릭 이벤트에 따라 메인 창의 포지션을 다시 계산한 뒤 표시합니다.
- **이유**: 트레이 아이콘 클릭 시 작업 표시줄의 위치를 고려해 우측 하단에 팝업을 띄우는 것이 가장 직관적입니다. 또한 Focus를 잃을 때 창을 닫는 로직(`tauri::WindowEvent::Focused(false)`)을 적용하여 자연스러운 팝업 동작을 만듭니다.

### 2. 위젯 창(Widget Window)의 분리
- **결정**: 기존 단일 메인 창(Main Window) 구조에서 벗어나, 위젯을 위한 별도의 논리적 윈도우(Widget Window)를 Tauri 런타임에 동적으로 생성합니다. 이 창은 `transparent: true`, `alwaysOnTop: true`, `decorations: false` 속성을 가집니다.
- **이유**: 메인 대시보드 창과 위젯은 생명주기와 스타일이 완전히 다릅니다. Frontend(React) 라우팅을 이용해 위젯 전용 URL(예: `/widget`)로 접근하게 하면, 리소스 관리가 용이합니다.

## Risks / Trade-offs

- **Risk: 트레이 클릭 이벤트가 OS별로 다르게 동작할 수 있음**
  - **Mitigation**: Windows 환경을 1순위 타겟으로 하므로, Windows의 작업 표시줄 동작과 위치 계산 로직에 먼저 집중하고 안정화합니다.
- **Risk: 다중 창 운영 시 React 상태 공유 문제**
  - **Mitigation**: 위젯 창은 단순히 백엔드에서 데이터를 읽어오기만(Read-only) 하도록 구성하거나, Tauri의 Store/Event(IPC) 시스템을 통해 상태 변경을 동기화합니다.
