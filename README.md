# maistats

`maistats`는 **maimaidx-eng.com** 데이터를 수집하고, 곡 메타데이터와 개인 플레이 기록을 각각 분리해서 제공하는 monorepo입니다. 저장소에는 Rust 서버 3개와 Vite/React 프런트엔드 1개가 들어 있습니다.

핵심 배포 모델은 다음과 같습니다.

- `maistats-song-info`: 개발자가 공용으로 호스팅
- `maistats-discord-bot`: 개발자가 공용으로 호스팅
- `apps/maistats`: 개발자가 공용으로 호스팅
- `maistats-record-collector`: 각 사용자가 자기 SEGA ID로 직접 호스팅

즉, **곡 정보는 공유되고 플레이 기록은 사용자별 self-hosted collector에서만 관리**됩니다.

## 구성 요소

### `maistats-song-info`

공용 곡 정보 서버입니다.

- 내부 `songdb` 서브시스템으로 곡 목록, 버전, 내부 레벨, 재킷 이미지를 준비합니다.
- `data/song_data/data.json`을 메모리로 로드해 API로 제공합니다.
- SongDB 관련 env가 설정돼 있으면 시작 시 업데이트를 시도하고, 이후 매일 **07:30 KST**에 다시 갱신합니다.
- 대표 엔드포인트:
  - `GET /health`
  - `GET /health/ready`
  - `GET /api/songs`
  - `GET /api/songs/versions`
  - `POST /api/songs/metadata`
  - `GET /api/cover/:image_name`

### `maistats-record-collector`

개인 플레이 기록 수집 서버입니다.

- 사용자의 SEGA ID로 로그인합니다.
- 프로세스별 임시 cookie store를 사용해 인증 세션을 유지합니다.
- SQLite를 사용하며 런타임에 `sqlx::migrate!()`로 마이그레이션을 적용합니다.
- 시작 시 점수 시드를 보장하고 `playerData`를 읽은 뒤, 플레이 횟수 변화가 있으면 recent를 동기화합니다.
- 이후 **10분마다** 백그라운드 polling을 수행합니다.
- 유지보수 시간대에는 초기 동기화를 건너뜁니다.
- 대표 엔드포인트:
  - `GET /health`
  - `GET /health/ready`
  - `GET /api/player`
  - `GET /api/scores/rated`
  - `GET /api/songs/scores`
  - `GET /api/recent`
  - `GET /api/today`
  - `GET /api/rating/targets`

### `maistats-discord-bot`

공용 Discord 봇입니다.

- 전역 Song Info 서버를 참조합니다.
- 각 Discord 사용자는 `/register <url>`로 자기 Record Collector URL을 등록합니다.
- 봇 자체 SQLite에 `Discord user -> record collector URL` 매핑을 저장합니다.
- 시작 시 slash command를 전역 등록하고, 개발자 계정에만 startup 요약 DM을 보냅니다.
- 주요 커맨드:
  - `/register`
  - `/mai-score`
  - `/mai-song-info`
  - `/mai-recent`
  - `/mai-today`

### `apps/maistats`

공용 웹 프런트엔드입니다.

- Vite + React 기반입니다.
- 기본적으로 Song Info 서버와 Record Collector 서버를 각각 다른 origin으로 붙습니다.
- Settings 화면에서 연결 URL을 직접 바꿀 수 있어, 공용 프런트엔드에서 각자 self-hosted collector에 붙는 방식으로 사용할 수 있습니다.
- 주요 화면:
  - Scores
  - Rating
  - Playlogs
  - Random Picker
  - Settings

## 저장소 구조

```text
.
|-- apps/maistats/                # Vite + React frontend
|-- maistats-song-info/           # shared song metadata server
|-- maistats-record-collector/    # per-user self-hosted record server
|-- maistats-discord-bot/         # shared Discord bot
`-- crates/
    |-- maimai-auth/              # maimaidx-eng.com auth helpers
    |-- maimai-parsers/           # HTML parsers
    `-- models/                   # shared API/domain/storage models
```

## 요구사항

- Rust stable
- Node.js 20+
- npm
- SEGA ID 계정
- Discord Bot Token 및 개발자 Discord User ID
- Song Info 갱신까지 사용할 경우 SongDB 관련 인증 정보와 `GOOGLE_API_KEY`

## 환경 변수

루트 `.env`는 Rust 서비스들이 공용으로 사용합니다.

```bash
cp .env.example .env
```

기본 항목:

- Record Collector
  - `SEGA_ID`
  - `SEGA_PASSWORD`
  - `RECORD_COLLECTOR_PORT`
  - `DATA_DIR`
  - `DATABASE_URL`
- Song Info
  - `SONG_INFO_PORT`
  - `SONG_DATA_PATH`
- Discord Bot
  - `DISCORD_BOT_TOKEN`
  - `DISCORD_DEV_USER_ID`
  - `SONG_INFO_SERVER_URL`
  - `DISCORD_BOT_DATABASE_URL`
- SongDB updater
  - `MAIMAI_INTL_SEGA_ID`
  - `MAIMAI_INTL_SEGA_PASSWORD`
  - `MAIMAI_JP_SEGA_ID`
  - `MAIMAI_JP_SEGA_PASSWORD`
  - `USER_AGENT`
  - `GOOGLE_API_KEY`

프런트엔드는 별도 `.env`를 사용합니다.

```bash
cp apps/maistats/.env.example apps/maistats/.env
```

- `SONG_INFO_SERVER_URL`
- `RECORD_COLLECTOR_SERVER_URL`

프런트엔드의 `RECORD_COLLECTOR_SERVER_URL`은 기본값일 뿐입니다. 실제 배포에서는 사용자가 Settings에서 자기 collector URL로 덮어쓰는 흐름을 전제로 합니다.

## 로컬 개발

의존성 설치:

```bash
npm ci
```

### 1. Song Info 서버 실행

```bash
cargo run -p maistats-song-info
```

기본 주소: `http://localhost:3001`

### 2. Record Collector 서버 실행

```bash
cargo run -p maistats-record-collector
```

기본 주소: `http://localhost:3000`

처음 실행 시 `data/`를 만들고 DB 마이그레이션을 적용합니다. 인증이나 초기 동기화가 실패해도 서버는 뜨므로, API 상태 확인과 디버깅을 분리해서 진행할 수 있습니다.

### 3. Discord 봇 실행

```bash
cargo run -p maistats-discord-bot
```

Discord 사용자는 `/register <record-collector-url>`로 자신의 collector를 연결해야 합니다.

### 4. 프런트엔드 실행

```bash
npm run dev:maistats
```

기본 주소: `http://localhost:5174`

## 운영 모델

권장 운영 형태는 다음과 같습니다.

1. 개발자가 `maistats-song-info`, `maistats-discord-bot`, `apps/maistats`를 공용으로 운영합니다.
2. 각 사용자는 자신의 환경에서 `maistats-record-collector`를 띄웁니다.
3. Discord에서는 `/register`로 collector URL을 등록합니다.
4. 웹에서는 Settings에서 collector URL을 입력해 같은 공용 프런트엔드에 연결합니다.

이 구조 덕분에 곡 정보와 UI는 한 곳에서 관리하면서도, 플레이 기록 DB와 로그인 세션은 각 사용자 환경에 남길 수 있습니다.

## 데이터 저장

- Record Collector
  - SQLite DB: 기본값 `data/maimai.sqlite3`
  - 임시 cookie file: OS temp 디렉터리 아래 `maistats-cookies-<pid>.json`
- Song Info
  - 곡 JSON: 기본값 `data/song_data/data.json`
  - 재킷 이미지: 기본값 `data/song_data/`
- Discord Bot
  - 봇 DB: 기본값 `data/maistats-discord-bot.sqlite3`

`data/`와 `.env`는 커밋하지 않습니다.

## 개발 점검 명령

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test
npm run build:maistats
```

## 배포 메모

- Rust 서비스는 각각 독립 바이너리로 배포합니다.
- 프런트엔드 `apps/maistats`는 Cloudflare용 Vite 설정을 사용합니다.
- CI/배포 워크플로는 `.github/workflows/` 아래에 있습니다.
