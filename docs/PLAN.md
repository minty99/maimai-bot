# maimai-bot 구현 계획서 (Rust, 단일 사용자)

이 문서는 `maimaidx-eng.com`(maimai DX NET, 영문 사이트)에서 **개인 플레이 기록을 수집/저장**하고, 이후 **자동 동기화(5분 간격)** 및 **Discord bot 조회**로 확장하기 위한 작업 계획서다.  
현재 단계의 목표는 **“페이지를 한 번 크롤링하는 모듈(로그인/쿠키 재사용 포함)”** 을 먼저 완성하는 것이다.

---

## 0. 목표/범위

### 최종 목표
- SEGA ID/PW로 로그인하여 세션을 확보하고 쿠키를 저장/재사용한다.
- 전곡 플레이 기록(난이도별)을 수집하여 DB(SQLite)에 저장한다.
- 주기적으로 최근 50회 플레이 기록을 수집하여 DB를 업데이트한다. (5분 간격, 자동화는 후속 단계)
- (후속) Discord bot에서 저장된 데이터를 조회한다.

### 사용자 범위
- 이 프로젝트는 **단일 사용자 전용**이며, **다중 사용자 지원은 하지 않는다**.

### 현재 우선순위 (이번 단계)
1) **로그인 + 쿠키 저장/재사용**이 되는 HTTP 클라이언트 구현  
2) 아래 URL을 **로그인 후 HTML로 저장**(= 크롤링 성공)  
   - `https://maimaidx-eng.com/maimai-mobile/record/` (최근 50회 플레이 페이지)
   - `https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff=<0..4>` (전곡 기록)
3) 전곡 기록 페이지를 파싱하여 “곡/난이도별 스코어”를 구조화 데이터로 뽑아낸다(우선 JSON 출력).  
4) (가능하면) SQLite에 upsert까지 연결한다.

### 비목표 (후속)
- Discord bot 구현
- 고급 분석(그래프, 추세, 통계 등)

---

## 1. 주요 제약/전략

### 로그인/세션 전략
- 기본 전략: **HTTP 기반**(`reqwest`)으로 로그인/세션 유지가 가능한지 먼저 시도한다.
- 실패 시(로그인이 JS 의존/추가 토큰 필요 등): **브라우저 자동화로 “로그인만”** 수행하고 쿠키를 추출하여 이후 HTTP 크롤링에 재사용한다.

### 데이터 키(곡 식별자) 전략
- 1차: 페이지에서 **musicId 등 안정적인 ID**(링크 파라미터/이미지 URL/hidden field)를 발견하면 그것을 `song_key`로 사용한다.
- 2차(임시): ID를 못 찾는다면 `title` 기반 키 사용.
  - 주의: `title`이 **빈 문자열일 수 있음** → `song_key`는 반드시 비어있지 않게 생성해야 한다.
  - 권장: `song_key = sha256(normalized_title)`  
    - `normalized_title = title.trim().to_lowercase()`  
    - `normalized_title`이 비어있다면 `sha256(jacket_image_url_or_row_html_or_row_text)` 같은 **대체 입력**으로 생성한다.
  - DB에는 `title`을 별도 컬럼으로 저장(빈 문자열 가능).

---

## 2. 추천 기술 스택 (Rust)

### 크롤링(HTTP) 레이어
- 런타임: `tokio`
- HTTP: `reqwest` (redirect/cookie 지원)
- 쿠키 저장/로드: `reqwest_cookie_store` + `cookie_store`
- HTML 파싱: `scraper` (CSS selector 기반)
- 설정: `dotenvy` (또는 `config`)
- CLI: `clap`
- 직렬화: `serde`, `serde_json`
- 로깅: `tracing`, `tracing-subscriber`
- 유틸: `eyre`(에러), `time`(timestamp), `sha2`(song_key 생성)

### DB 레이어 (SQLite)
단일 사용자/로컬 파일 환경이면 SQLite가 가장 단순하다.
- 추천 1: `sqlx` + SQLite
  - 장점: upsert/마이그레이션/쿼리 작성이 편하고, 이후 async 구성과 자연스럽게 맞는다.
  - 단점: 컴파일 옵션/기능 플래그가 필요.
- 추천 2: `rusqlite`
  - 장점: 의존성/구성이 단순, 로컬 도구 느낌으로 빠르게 만들기 좋다.
  - 단점: async와는 별개(필요 시 별도 스레드 풀/채널로 감싸야 함).

**권장 선택**: 이후 5분 주기 자동화 및 향후 Discord bot(네트워크 I/O)까지 고려하면 `sqlx`를 권장한다.

### 브라우저 자동화(로그인 fallback)
HTTP 로그인 시도가 실패할 경우에만 도입한다.
- WebDriver: `thirtyfour` + `chromedriver`
  - 실행 환경에 크롬/크로미움과 chromedriver 필요
  - 로그인 후 쿠키를 읽어 `cookie_store` 포맷으로 저장

---

## 3. 프로젝트 구조 제안

초기에는 단일 바이너리로 시작하고, 모듈로 분리한다.

```
src/
  main.rs
  config.rs
  cli.rs
  http/
    mod.rs
    client.rs          # reqwest + cookie_store 래핑
    auth.rs            # 로그인 감지/수행
  maimai/
    mod.rs
    endpoints.rs       # URL/쿼리 생성
    models.rs          # ScoreRow 등 데이터 모델(serde)
    parse/
      mod.rs
      score_list.rs    # 전곡 기록 파서
      recent.rs        # 최근 50회 파서(후속)
  db/
    mod.rs
    schema.sql         # (sqlx면 migrations로 대체)
    repo.rs            # insert/upsert/query
docs/
  PLAN.md
data/
  (gitignored) cookies.json, maimai.sqlite3, raw/*.html ...
```

---

## 4. CLI 설계 (초기부터 유용하게)

초기 개발/디버깅을 쉽게 하기 위해 CLI를 먼저 잡아두면 좋다.

### 커맨드 제안
- `maimai-bot auth check`
  - 쿠키 로드 후 보호 페이지 1개 요청 → 로그인 필요 여부 출력
- `maimai-bot auth login`
  - 강제 로그인 수행 → 쿠키 저장
- `maimai-bot fetch url --url <URL> --out data/raw/page.html`
  - 인증 포함 GET → 응답 HTML 저장(디버깅용)
- `maimai-bot crawl scores --diff <0..4|all> --out data/out/scores.json`
  - 전곡 기록 페이지 크롤링+파싱 결과를 JSON으로 출력
- `maimai-bot db init`
  - SQLite 파일/테이블 초기화
- `maimai-bot db import-scores --in data/out/scores.json`
  - 파싱 결과를 DB에 upsert

자동화(스케줄링)는 후속 단계에서 `maimai-bot sync run` 형태로 추가.

---

## 5. 인증/쿠키 동작 설계

### 쿠키 저장 위치
- `data/cookies.json` (gitignore 처리)

### 로그인 필요 여부 감지
요청한 URL이 다음 중 하나면 “로그인 필요”로 판단:
- 응답 URL이 로그인 페이지로 redirect 됨
- HTML에 로그인 폼/특정 문구가 존재(예: `SEGA ID`, `login`, `authenticate` 등)
- `Set-Cookie`가 비정상(세션 없음) + 페이지 내용이 보호됨

### HTTP 로그인 플로우(우선 시도)
- 로그인 페이지 GET → 필요한 hidden field/토큰이 있으면 파싱
- 로그인 POST(폼 전송) → 성공 여부 확인(보호 페이지 접근 가능 여부)
- 성공 시 cookie jar 저장

### 브라우저 fallback 플로우(필요 시)
- `chromedriver` 실행
- 로그인 페이지 접속 → ID/PW 입력 → submit → 로그인 완료 페이지 도달 확인
- WebDriver로 쿠키 목록을 가져와 `cookie_store`에 주입 → `data/cookies.json` 저장
- 이후는 HTTP 클라이언트로 동일하게 진행

---

## 6. 수집 대상 데이터와 필드 정의

### 전곡 기록(난이도별) URL
- `https://maimaidx-eng.com/maimai-mobile/record/musicGenre/search/?genre=99&diff=<diff_value>`
  - `diff_value`: 0 BASIC, 1 ADVANCED, 2 EXPERT, 3 MASTER, 4 Re:MASTER

### 저장하고 싶은 필드(초기)
- 달성률(percentage)
- 등급(SSS+ 등)
- FC/AP 여부
- SYNC 관련 정보(예: FS/FS+/FDX 등 사이트 표기 기준)
- DX 점수

### 파서 출력 모델(예시)
`ScoreEntry` (전곡 기록 한 행)
- `song_key: String` (비어있지 않게 생성)
- `title: String` (빈 문자열 가능)
- `diff: u8` (0..4)
- `achievement_percent: Option<f32>`
- `rank: Option<String>` (예: "SSS+")
- `fc: Option<String>` (예: "FC", "FC+", "AP", "AP+" 등 원문 그대로 보존)
- `sync: Option<String>` (예: "FS", "FS+", "FDX", "FDX+" 등 원문 그대로 보존)
- `dx_score: Option<i32>`
- `scraped_at: i64` (unix timestamp)
- `source_url: String`

처음엔 “원문 그대로 보존”이 안전하다. 이후 정규화(enum)로 바꿔도 된다.

---

## 7. DB 스키마 (초안)

### 파일 위치
- `data/maimai.sqlite3`

### 테이블 제안(최소)

#### `songs`
- `song_key TEXT PRIMARY KEY` (프로젝트 내부의 안정키: ID 또는 해시)
- `title TEXT NOT NULL` (빈 문자열 허용; NULL 대신 빈 문자열로 저장하는 편이 단순)
- `created_at INTEGER NOT NULL`
- `updated_at INTEGER NOT NULL`

#### `scores`
- `song_key TEXT NOT NULL`
- `diff INTEGER NOT NULL` (0..4)
- `achievement_percent REAL` (NULL 가능)
- `rank TEXT`
- `fc TEXT`
- `sync TEXT`
- `dx_score INTEGER`
- `scraped_at INTEGER NOT NULL`
- `PRIMARY KEY (song_key, diff)`
- `FOREIGN KEY (song_key) REFERENCES songs(song_key)`

#### `sync_state` (후속 자동화 대비)
- `key TEXT PRIMARY KEY`
- `value TEXT NOT NULL`

### upsert 규칙
- `songs`: 같은 `song_key`면 title이 빈 문자열인데 새 title이 유효하면 갱신
- `scores`: `(song_key, diff)` 기준 upsert, `scraped_at` 최신값으로 갱신

---

## 8. 단계별 작업 티켓(마일스톤)

### M1. “인증 포함 fetch” 완성 (DB 없이도 OK)
**산출물**
- 쿠키를 `data/cookies.json`으로 저장/로드 가능
- `record/` 및 `musicGenre/search` HTML을 파일로 저장 가능

**세부 작업**
1. `.env` 로드 (`SEGA_ID`, `SEGA_PASSWORD`)
2. `HttpClient` 구현
   - 기본 헤더(User-Agent 등) 설정
   - cookie store 로드/세팅
   - 요청/응답 로깅(최소 URL, status)
3. `login_required()` 판별 로직
4. `login()` 구현(HTTP 시도)
5. `fetch_to_file(url, out_path)` 구현
6. 실패 시: 로그인 폼 구조/hidden field 확인을 위해 `data/raw/login.html` 저장 옵션 제공

**검증 체크리스트**
- 최초 실행: 쿠키 없음 → 로그인 수행 → 쿠키 저장됨
- 재실행: 쿠키 있음 → 로그인 없이 보호 페이지 접근됨
- 쿠키 만료 가정: 보호 페이지 접근 실패 시 자동 재로그인 후 성공

### M2. 전곡 기록 파싱(난이도별)
**산출물**
- `diff=0..4` 각각 파싱하여 `Vec<ScoreEntry>` 생성
- JSON 출력 파일(`data/out/scores.json`) 생성

**세부 작업**
1. `endpoints.rs`에 diff URL 생성 함수
2. `score_list.rs` 파서 구현
   - selector 기반으로 곡 제목/기록 필드 추출
   - 파싱 실패한 행은 “원문 텍스트”를 함께 로그로 남기고 skip(개발 단계)
3. `song_key` 생성 규칙 구현(빈 title 처리 포함)
4. `crawl scores --diff all` 커맨드 구현

**검증 체크리스트**
- diff별 곡 개수/중복 여부 확인(대략적인 합리성)
- `title == ""`인 항목이 있어도 crash 없이 `song_key` 생성됨
- 퍼센트/점수 파싱 실패 시 `Option` 처리로 안전하게 통과

### M3. SQLite 저장(Upsert)
**산출물**
- `db init`로 스키마 생성
- `db import-scores`로 `songs/scores` upsert

**세부 작업**
1. `sqlx` 또는 `rusqlite` 선택 및 초기화
2. 마이그레이션/스키마 파일 작성
3. `upsert_song`, `upsert_score` 구현
4. import 시 트랜잭션 사용(성능/일관성)

**검증 체크리스트**
- 동일 JSON을 2번 import해도 row 수가 폭증하지 않음(업서트 확인)
- `scraped_at`이 갱신됨

### M4. 최근 50회 플레이 파싱/저장 (후속)
**산출물**
- `record/` 페이지에서 최근 플레이 리스트 파싱
- `playlogs` 테이블(필요 시) 추가 및 중복 방지 키 설계

**핵심 애매점(후속에서 확인할 것)**
- 플레이 1건의 고유 ID/시간/곡/난이도 등이 HTML에 어떻게 존재하는지
- 중복 방지 키를 무엇으로 할지(가능하면 사이트 제공 ID, 없으면 timestamp+song+diff 조합)

### M5. 자동 동기화(5분) (후속)
- `sync run --interval 5m` 혹은 OS 스케줄러(cron/launchd/systemd)로 실행
- 실패 시 백오프/재로그인/로그 기록

### M6. Discord bot (후속)
- 조회 API 레이어를 먼저 만들고, Discord 명령은 thin wrapper로 연결

---

## 9. 운영/안전(필수)

### 시크릿/데이터 파일
- `.env`, `data/`는 git에 커밋하지 않는다.
- 쿠키/DB는 민감정보 취급(공유 금지).

### 레이트리밋/차단 회피(후속 자동화 시)
- 5분 간격이라도 한 번의 sync에서 여러 페이지를 요청할 수 있음 → 요청 사이에 짧은 지연(예: 200~500ms) 추가 고려
- 실패 시 즉시 무한 재시도 금지(지수 백오프)

### 파서 내구성
- 사이트 HTML은 언제든 바뀔 수 있음 → 파싱 실패 시 원본 HTML 저장 옵션은 필수
- selector가 깨졌을 때 빠르게 수정 가능하도록 “샘플 HTML”을 `data/raw/`에 보관(개발 중에만)

---

## 10. “다음 작업자”를 위한 시작 가이드

0) 의존성 추가/제거
   - `Cargo.toml`을 직접 편집하기보다 `cargo add`, `cargo remove`를 우선 사용한다.

1) `.env` 준비
   - `SEGA_ID=...`
   - `SEGA_PASSWORD=...`
2) 개발 실행(예시)
   - `cargo run -- auth login`
   - `cargo run -- fetch url --url "https://maimaidx-eng.com/maimai-mobile/record/" --out data/raw/record.html`
   - `cargo run -- crawl scores --diff all --out data/out/scores.json`
3) (DB까지 할 경우)
   - `cargo run -- db init`
   - `cargo run -- db import-scores --in data/out/scores.json`

브라우저 fallback이 필요해지면:
- `chromedriver` 준비/실행 후 `maimai-bot auth login --via-webdriver` 같은 옵션을 추가한다.
