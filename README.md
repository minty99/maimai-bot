# maimai-bot

`maimai-bot`은 **maimai DX NET (maimaidx-eng.com)** 에 로그인(SEGA ID)해서 기록을 크롤링하고, 로컬 SQLite에 저장한 뒤 Discord에서 조회/알림을 받는 **단일 사용자** monorepo입니다.

이 저장소는 Rust 런타임 3개와 Cloudflare Pages로 배포하는 `maistats` 프론트엔드를 함께 관리합니다.

## 아키텍처

이 프로젝트는 **두 개의 독립적인 서버** + **Discord 봇** + **웹 프론트엔드** 구조로 분리되어 있습니다:

- **Song Info Server** (`song-info-server/`): 공개 곡 정보 제공 (stateful updater + API)
  - 시작 시 곡 데이터가 없으면 `maimai-songdb`로 곡/내부레벨/재킷 정보를 가져와 `data/song_data/`에 저장합니다.
  - 매일 07:30 KST에 곡 데이터를 다시 갱신하고 메모리에 리로드합니다.
  - 저장된 JSON(`data/song_data/data.json`)을 로드해 API로 제공합니다.
  - 재킷 이미지를 정적 파일로 서빙합니다.
  - 포트: `3001` (기본값)
  - 의존성: SongDB fetch용 환경 변수 (`MAIMAI_*`, `GOOGLE_API_KEY`) 필요

- **Record Collector Server** (`record-collector-server/`): 개인 기록 수집 및 관리 (인증 필요, stateful)
  - 쿠키를 `data/` 아래에 저장/재사용하고, 만료 시 재로그인해서 갱신합니다.
  - DB는 `sqlx::migrate!()`로 런타임에 마이그레이션을 실행합니다.
  - 곡 메타데이터(레벨/내부레벨/버전 등)는 저장하지 않으며, 플레이/스코어 기록만 수집합니다.
  - 시작 시 `playerData`를 크롤링하고, 필요하면 scores(난이도 0..4) + recent를 DB에 초기 적재합니다.
  - 이후 10분마다 `playerData`를 다시 크롤링해서 **total play count 변화가 있을 때만** recent를 크롤링합니다.
  - 포트: `3000` (기본값)
  - 의존성: SEGA ID 인증 정보

- **Discord Bot** (`personal-discord-bot/`): 두 서버의 API를 호출하여 Discord 명령어 처리 및 DM 알림 전송
  - Record Collector Server의 `/health/ready` 엔드포인트를 폴링하여 서버가 준비될 때까지 대기합니다.
  - Record Collector Server에서 새 플레이가 감지되면 DM으로 알림을 보냅니다.
  - 의존성: Song Info Server + Record Collector Server

- **maistats** (`apps/maistats/`): `song-info-server`와 `record-collector-server` 데이터를 탐색하는 Vite + React 웹 UI
  - Cloudflare Pages 배포 대상입니다.
  - 기본 API origin은 `SONG_INFO_SERVER_URL`, `RECORD_COLLECTOR_SERVER_URL` 환경 변수로 주입합니다.
  - 로컬에서는 브라우저 UI 설정으로 origin을 덮어쓸 수 있습니다.

**중요**: Song Info Server와 Record Collector Server를 먼저 실행한 후 Discord 봇을 실행해야 합니다.

## 특징

- 쿠키 기반 인증으로 SEGA ID 로그인 유지
- SQLite 기반 로컬 데이터 저장
- 자동 스코어 동기화 및 플레이 로그 추적
- Discord 슬래시 커맨드 및 DM 알림

## 요구사항

- Rust (stable)
- Node.js 20+ / npm 10+ (`apps/maistats` 개발 시)
- Discord Bot Token / 단일 수신자(User ID) (Discord 봇 사용 시)
- SEGA ID 계정 (maimaidx-eng.com) (Record Collector Server 사용 시)
- Song Info Server SongDB 갱신용 환경 변수 (`MAIMAI_*`, `GOOGLE_API_KEY`, `USER_AGENT`)

## 설정

환경 변수는 프로젝트 루트의 `.env` 파일을 사용합니다:

```bash
cp .env.example .env
# 편집: SEGA_ID, SEGA_PASSWORD, DISCORD_BOT_TOKEN, DISCORD_USER_ID 등 입력
```

**주의**:
- `.env` 파일은 절대 커밋하지 마세요 (`.gitignore`에 포함됨)
- `dotenvy`가 상위 디렉토리를 자동 탐색하므로 어디서 실행하든 작동합니다
- **보안**: 각 서비스는 필요한 환경 변수만 로드합니다 (Least Privilege 원칙)

### 기본 런타임 경로

- Song Info Server:
  - 곡 데이터: `data/song_data/data.json` (기본값)
  - 재킷 이미지: `data/song_data/cover/`
- Record Collector Server:
  - DB: `data/maimai.sqlite3`
  - 쿠키: `data/cookies.json`

## 실행

### Monorepo 의존성 설치

프런트엔드를 빌드하거나 실행할 때는 저장소 루트에서 npm workspace 의존성을 설치합니다.

```bash
npm ci
```

### Standalone 실행 (로컬 개발)

**반드시 두 서버를 먼저 실행한 후 Discord 봇을 실행하세요.**

1. **환경 변수 설정** (처음 한 번만):
   ```bash
   cp .env.example .env
   # .env 파일 편집하여 실제 credentials 입력
   ```

2. **Song Info Server 실행** (터미널 1):
   ```bash
   cargo run --bin song-info-server
   ```
   Song Info Server는 `http://localhost:3001`에서 실행되며, 곡 정보 및 재킷 이미지를 제공합니다.

3. **Record Collector Server 실행** (터미널 2):
   ```bash
   cargo run --bin record-collector-server
   ```
   Record Collector Server는 `http://localhost:3000`에서 실행되며, `/health/ready` 엔드포인트를 제공합니다.

4. **Discord 봇 실행** (터미널 3):
   ```bash
   cargo run --bin personal-discord-bot
   ```
   Discord 봇은 Record Collector Server의 `/health/ready`를 폴링하여 서버가 준비될 때까지 대기합니다.

**참고**:
- `dotenvy`가 현재 디렉토리와 상위 디렉토리를 탐색하므로 프로젝트 루트에서 실행해도 각 서비스의 `.env`를 자동으로 찾습니다.
- Song Info Server는 독립적으로 실행 가능하며, Record Collector Server나 Discord 봇 없이도 사용할 수 있습니다.

### maistats 실행

서버 API가 올라와 있다면 루트에서 다음 명령으로 프런트엔드를 실행할 수 있습니다.

```bash
cp apps/maistats/.env.example apps/maistats/.env
npm run dev:maistats
```

프로덕션 빌드는 다음 명령으로 생성합니다.

```bash
npm run build:maistats
```

빌드 결과물은 `apps/maistats/dist/`에 생성됩니다.

### 개발/디버깅 명령어

Record Collector Server에서 제공하는 CLI 명령어들 (레거시, 참고용):

쿠키 로그인/체크:
- `cargo run --bin record-collector-server -- auth login`
- `cargo run --bin record-collector-server -- auth check`

HTML/raw fetch (로그인 필요):
- `cargo run --bin record-collector-server -- fetch url --url https://maimaidx-eng.com/maimai-mobile/playerData/ --out data/out/player_data.html`

크롤링/파싱(JSON)만 수행 (DB 미사용):
- `cargo run --bin record-collector-server -- crawl player-data --out data/out/player_data.json`
- `cargo run --bin record-collector-server -- crawl recent --out data/out/recent.json`
- `cargo run --bin record-collector-server -- crawl scores --out data/out/scores.json`

## Discord 명령어

- `/mai-score <title>`
  - 곡 제목 exact match만 조회합니다.
  - exact match가 없으면 조회 실패로 처리합니다.
  - 기록이 없는(미플레이) 항목은 출력하지 않습니다.
- `/mai-recent`
  - recent 페이지 기준 “가장 최근 1 credit”만 보여줍니다.
  - 맨 앞의 `TRACK 01`을 기준으로 그 credit의 플레이들을 `TRACK 01 -> ...` 순서로 출력합니다.

## 데이터 모델/저장 방식 (요약)

- `scores` / `playlogs`에 `achievement_x10000`으로 저장합니다 (`percent * 10000`, 반올림).
- 난이도/차트/랭크/FC/SYNC는 문자열 아이콘을 enum으로 파싱하지만, DB에는 표시용 문자열(TEXT)로 저장합니다.

## 배포

- Docker 이미지: GitHub Actions가 `song-info-server`, `record-collector-server`, `personal-discord-bot` 3개 이미지만 빌드/배포합니다.
- `maistats`: Cloudflare Pages가 monorepo 루트에서 `npm ci && npm run build --workspace apps/maistats`를 실행하고, 출력 디렉터리는 `apps/maistats/dist`를 사용합니다.
