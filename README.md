# FileMoa

FileMoa는 Windows에서 선택한 폴더를 안전하게 정리하는 한국어·영어 데스크톱
앱입니다. 파일을 바로 삭제하는 클리너가 아니라 `분석 → 추천 → 미리보기 →
승인 → 이동 → 기록 → Undo` 흐름을 지킵니다.

## 현재 구현된 기능

- 바탕화면·다운로드·문서·사진·동영상·음악 빠른 선택, 사용자 폴더 선택,
  현재 폴더/하위 폴더 스캔, 여러 폴더 동시 계획(기본 꺼짐)
- PDF, Word, Excel, PowerPoint, HWP/HWPX, 텍스트, 전자책, 이미지·스크린샷,
  동영상, 오디오, 압축파일, 설치파일, 개발파일, 폰트, 디스크 이미지, 데이터,
  기타 분류
- 파일 형식·생성/수정/접근 날짜·연도·크기·파일명 IF→THEN 기준
- 사용자 카테고리와 확장자, 숨김 파일/예외 설정, 개발 프로젝트 보호
- 충돌 시 `(1)`, `(2)`를 붙이는 Dry Run 미리보기와 오류/제외 목록
- 파일 이동, 실행 기록(최근 30일), 충돌 없는 Undo 미리보기
- 크기 선필터 후 전체 SHA-256 중복 탐색, 사용자 지정 크기의 대용량 파일·빈 폴더 후보 탐색
- 선택한 항목을 Windows Recycle Bin으로 보내는 명시적 확인 작업
- OpenAI, Anthropic Claude, Google Gemini API, localhost Ollama를 매 실행마다
  선택하는 AI 추천. AI는 파일명(베이스네임)·확장자·크기·시간만 전송하고 파일
  내용/전체 경로를 보내지 않으며, 제안은 사용자가 승인한 뒤 미리보기에 반영됩니다.
- API 키는 Windows DPAPI로 현재 사용자 계정에 묶어 암호화 저장하며 내보내기
  기능이 없습니다. 설정 화면의 공식 키 발급 링크와 모델 ID 편집을 제공합니다.
- 시작 시 조용한 업데이트 확인(배지만 표시), 설정에서 수동 확인 시 버전·릴리즈
  노트와 `지금 설치`/`나중에 설치`를 보여주는 사용자 제어 업데이트

## 로컬 실행

```powershell
Set-Location F:\Codex-F\smartfile
npm ci
npm run tauri dev
```

## 검증

```powershell
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo check --manifest-path src-tauri\Cargo.toml -j1 --lib
```

Windows 러스트 테스트는 Tauri의 대형 `windows` 의존성을 컴파일하므로 메모리가
작은 환경에서는 `-j1`로 실행하는 것을 권장합니다. `cargo check`와 프런트엔드
테스트/빌드는 릴리즈 전 필수 확인 항목입니다.

## NSIS 설치 파일

사용자가 릴리즈 빌드를 요청한 경우에만 다음 명령을 실행합니다.

```powershell
npm run build:nsis
```

결과는 `src-tauri\target\release\bundle\nsis\`에 생성됩니다. 현재는
Authenticode 인증서를 연결하지 않은 설치 파일이므로 첫 실행 때 Windows
SmartScreen 경고가 나타날 수 있습니다. 인증서를 추가하면 해당 경고를 줄일 수
있지만, 서명 없이 완전히 숨기는 것은 Windows 보안 정책상 보장할 수 없습니다.

## GitHub Releases 자동 업데이트

`.github/workflows/release.yml`은 `v*` 태그가 올라올 때만 Windows NSIS와 Tauri
업데이트 서명 아티팩트를 만듭니다. 업로더는 다음 비밀을 필요로 합니다.

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (키에 비밀번호를 설정한 경우)

업데이트 서명은 설치 파일의 Authenticode 서명과 별개입니다. 앱은 시작 시 자동
설치하지 않으며, 사용자가 설정에서 확인한 후 모달의 `지금 설치`를 눌러야 합니다.

## AI 공급자 링크

- [OpenAI API keys](https://platform.openai.com/api-keys)
- [Anthropic API keys](https://console.anthropic.com/settings/keys)
- [Google AI Studio API key](https://aistudio.google.com/app/apikey)
- [Ollama for Windows](https://ollama.com/download/windows)

Ollama는 `127.0.0.1:11434`만 확인하고 설치된 모델을 자동 다운로드하지 않습니다.

## 보안 원칙

시스템 폴더, AppData, 개발 프로젝트(`.git`, `package.json`, `Cargo.toml` 등),
임시 다운로드와 숨김 파일은 기본 제외합니다. 덮어쓰기와 영구 삭제는 사용하지
않고, 오류와 충돌은 계획/기록에 남깁니다.
