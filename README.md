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
- 파일 형식·생성/수정/접근 날짜(년-월·년-월-일)·연도·크기·파일명(가나다·ABC)·파일명 키워드 IF→THEN 기준
- 사용자 카테고리와 확장자, 숨김 파일/예외 설정, 개발 프로젝트 보호(`.git` 등 저장소는 단독으로, `package.json` 같은 파일은 둘 이상이거나 소스 폴더와 함께일 때)
- 충돌 시 `(1)`, `(2)`를 붙이는 Dry Run 미리보기와 오류/제외 목록
- 파일 이동, 실행 기록(최근 30일), 충돌 없는 Undo 미리보기. 이동마다 저널에 기록해
  앱이 도중에 종료돼도 다음 실행에서 복구하고 되돌릴 수 있으며, Undo는 이번 실행이
  만든 빈 폴더도 정리합니다.
- 큰 폴더에서도 창이 멈추지 않는 백그라운드 실행, 진행률 표시와 취소(정리 실행은
  지금 옮기는 파일까지만 마치고 멈추며 되돌릴 수 있음)
- 크기 → 앞 64KB → 전체 SHA-256 순서의 중복 탐색(병렬), 사용자 지정 크기의
  대용량 파일·빈 폴더 후보 탐색. OneDrive/Drive의 온라인 전용 파일은 내려받지 않도록 제외
- 검사 결과에 있던 항목만 Windows Recycle Bin으로 보내는 명시적 확인 작업.
  중복 그룹의 마지막 복사본은 지울 수 없고, 휴지통이 없는 네트워크·이동식 드라이브의
  항목은 영구 삭제되지 않도록 거부합니다.
- OpenAI, Anthropic Claude, Google Gemini API, localhost Ollama를 매 실행마다
  선택하는 AI 추천. AI는 파일명(베이스네임)·확장자·크기·시간만 전송하고 파일
  내용/전체 경로를 보내지 않으며, 제안은 사용자가 승인한 뒤 미리보기에 반영됩니다.
- API 키는 Windows DPAPI로 현재 사용자 계정에 묶어 암호화 저장하며 내보내기
  기능이 없습니다. 설정 화면의 공식 키 발급 링크와 모델 ID 편집을 제공합니다.
- 시작 시 조용한 업데이트 확인(배지만 표시), 설정에서 수동 확인 시 버전·릴리즈
  노트와 `지금 설치`/`나중에 설치`를 보여주는 사용자 제어 업데이트
- 현재 사용자용 NSIS 설치 후 탐색기 폴더·폴더 배경·드라이브와 바탕화면의
  오른쪽 클릭 메뉴에서 FileMoa를 실행하고 해당 경로를 자동 선택

## 로컬 실행

```powershell
Set-Location <저장소 폴더>
npm ci
npm run tauri dev
```

Google Drive·OneDrive처럼 동기화되는 폴더에서는 `npm ci`와 Rust 빌드가 파일 잠금으로
실패할 수 있으니 로컬 디스크의 폴더에서 작업하세요.

## 검증

```powershell
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
cargo clippy --manifest-path src-tauri\Cargo.toml --lib --tests -- -D warnings
cargo test --manifest-path src-tauri\Cargo.toml
```

`.github/workflows/ci.yml`이 `main`을 제외한 모든 브랜치와 풀 리퀘스트에서 같은 검사를
실행합니다. 메모리가 작은 환경에서는 Rust 명령에 `-j1`을 붙이세요. 휴지통에 실제
파일 하나를 보내는 통합 테스트는 기본 제외이며
`cargo test --lib -- --ignored recycles_a_real_file`로 따로 실행합니다.

수동 확인 항목(영구 삭제 경고, 실제 클라우드 폴더, 앱 두 개 동시 실행)은
`specs/005-safety-hardening/beta-test.md`에 정리되어 있습니다.

## NSIS 설치 파일

사용자가 릴리즈 빌드를 요청한 경우에만 다음 명령을 실행합니다.

```powershell
npm run build:nsis
```

결과는 `src-tauri\target\release\bundle\nsis\`에 생성됩니다. 현재는
Authenticode 인증서를 연결하지 않은 설치 파일이므로 첫 실행 때 Windows
SmartScreen 경고가 나타날 수 있습니다. 인증서를 추가하면 해당 경고를 줄일 수
있지만, 서명 없이 완전히 숨기는 것은 Windows 보안 정책상 보장할 수 없습니다.

릴리즈 빌드는 `rust-toolchain.toml`에 지정한 Rust 1.94.1을 사용합니다. Tauri의
NSIS 실행 파일 패치 단계에서 최신 툴체인과의 호환 문제가 해결되기 전까지
설치 후 즉시 종료되는 회귀를 막기 위해 CI와 로컬 빌드에 같은 버전을 적용합니다.

설치 훅이 현재 사용자 레지스트리에 셸 메뉴를 등록하고 제거 시 FileMoa 항목만
삭제합니다. Windows 11의 간소화된 메뉴에서 보이지 않으면 `추가 옵션 표시`에서
찾을 수 있습니다. 셸 메뉴를 사용하려면 설치 프로그램을 새로 실행해야 합니다.
셸에서 실행된 FileMoa는 `--organize` 인자로 받은 폴더를 시작 화면에 표시하지만,
기존 미리보기·승인 절차 전에는 파일을 이동하지 않습니다.

## GitHub Releases 자동 업데이트

`.github/workflows/release.yml`은 `main`에 커밋이 푸시되거나 `v*` 태그가 올라오면
테스트를 실행하고, **그 버전(`v<version>`)의 릴리즈가 아직 없을 때만** Windows NSIS와
Tauri 업데이트 서명 아티팩트를 빌드해 GitHub Release로 올립니다. 배포하려면
`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`의 버전을 같이
올린 커밋을 푸시하세요. 같은 버전으로 다시 푸시하면 검사만 통과시키고 이미 배포된
설치 파일과 `latest.json`을 덮어쓰지 않습니다(수동 실행의 `force` 옵션으로만 재배포).

`0.2.0-beta.1`처럼 하이픈이 있는 버전은 **사전 릴리즈**로 올라갑니다. 업데이터가 따르는
`latest` 릴리즈에는 포함되지 않으므로 기존 사용자에게 자동 업데이트되지 않고, 설치 파일을
직접 내려받아 쓰는 사람만 받습니다. 정식 버전(`0.2.0`)을 올리면 모든 사용자에게 업데이트가
제공됩니다. 업로더는 다음 비밀을 필요로 합니다.

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

시스템 폴더, AppData, 개발 프로젝트, 임시 다운로드와 숨김 파일은 기본 제외하고, 드라이브
루트·사용자 폴더 전체·휴지통 폴더는 정리 대상으로 선택할 수 없습니다(8.3 짧은 이름이나
링크로 우회해도 실제 경로로 풀어 검사). 덮어쓰기와 영구 삭제는 사용하지 않고, 오류와
충돌은 계획/기록에 남깁니다.

- 화면은 경로를 보내지 않습니다. 정리 계획은 백엔드가 보관하고 화면은 계획 ID만, 되돌리기는
  실행 기록 ID만 보냅니다. 실행 직전에 모든 항목이 선택한 폴더 안인지, 보호 경로가 아닌지,
  미리보기 이후 바뀌지 않았는지 다시 확인합니다.
- 휴지통 작업은 검사 결과에 있던 항목에만, 검사 이후 바뀌지 않았을 때만 허용합니다.
- AI에는 파일명(베이스네임)·확장자·크기·시간만 보내고, 파일명을 지시문이 아닌 데이터로
  취급하도록 프롬프트에 명시합니다. 제안은 사용자가 승인해야 반영되며 새 카테고리는 실행당
  12개로 제한합니다. Gemini 키는 URL이 아니라 헤더로 보내고, 저장된 키는 DPAPI에 앱 고유
  엔트로피를 더해 암호화합니다.
