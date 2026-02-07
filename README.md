# maimai-bot

`maimai-bot`은 **maimai DX NET (maimaidx-eng.com)** 에 로그인(SEGA ID)해서 기록을 크롤링하고, 로컬 SQLite에 저장한 뒤 Discord에서 조회/알림을 받는 **단일 사용자** Rust 앱입니다.

## 아키텍처

이 프로젝트는 **두 개의 독립적인 서버** + **Discord 봇** 구조로 분리되어 있습니다:

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
  - 시작 시 `playerData`를 크롤링하고, 필요하면 scores(난이도 0..4) + recent를 DB에 초기 적재합니다.
  - 이후 10분마다 `playerData`를 다시 크롤링해서 **total play count 변화가 있을 때만** recent를 크롤링합니다.
  - 포트: `3000` (기본값)
  - 의존성: Song Info Server (곡 정보 조회용)

- **Discord Bot** (`personal-discord-bot/`): 두 서버의 API를 호출하여 Discord 명령어 처리 및 DM 알림 전송
  - Record Collector Server의 `/health/ready` 엔드포인트를 폴링하여 서버가 준비될 때까지 대기합니다.
  - Record Collector Server에서 새 플레이가 감지되면 DM으로 알림을 보냅니다.
  - 의존성: Song Info Server + Record Collector Server

**중요**: Song Info Server와 Record Collector Server를 먼저 실행한 후 Discord 봇을 실행해야 합니다.

## 특징

- 쿠키 기반 인증으로 SEGA ID 로그인 유지
- SQLite 기반 로컬 데이터 저장
- 자동 스코어 동기화 및 플레이 로그 추적
- Discord 슬래시 커맨드 및 DM 알림

## 요구사항

- Rust (stable)
- Discord Bot Token / 단일 수신자(User ID) (Discord 봇 사용 시)
- SEGA ID 계정 (maimaidx-eng.com) (Record Collector Server 사용 시)
- Song Info Server SongDB 갱신용 환경 변수 (`MAIMAI_*`, `GOOGLE_API_KEY`, `USER_AGENT`)

## 설정

환경 변수는 **실행 모드에 따라 다른 파일**을 사용합니다:

### Standalone 개발 (로컬 실행)

각 서비스별로 `.env` 파일을 생성하세요:

**1. Song Info Server 설정** (`song-info-server/.env`)
```bash
cp song-info-server/.env.example song-info-server/.env
# 편집: SONG_INFO_PORT, SONG_DATA_PATH, MAIMAI_*, USER_AGENT, GOOGLE_API_KEY 등 입력
```

**2. Record Collector Server 설정** (`record-collector-server/.env`)
```bash
cp record-collector-server/.env.example record-collector-server/.env
# 편집: SEGA_ID, SEGA_PASSWORD, DATABASE_URL, SONG_INFO_SERVER_URL 등 입력
```

**3. Discord Bot 설정** (`personal-discord-bot/.env`)
```bash
cp personal-discord-bot/.env.example personal-discord-bot/.env
# 편집: DISCORD_BOT_TOKEN, DISCORD_USER_ID, SONG_INFO_SERVER_URL, RECORD_COLLECTOR_SERVER_URL 등 입력
```

### Docker Compose (프로덕션)

루트 `.env` 파일을 생성하세요:

```bash
cp .env.example .env
# 편집: 모든 환경 변수 입력
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

### Standalone 실행 (로컬 개발)

**반드시 두 서버를 먼저 실행한 후 Discord 봇을 실행하세요.**

1. **환경 변수 설정** (처음 한 번만):
   ```bash
   cp song-info-server/.env.example song-info-server/.env
   cp record-collector-server/.env.example record-collector-server/.env
   cp personal-discord-bot/.env.example personal-discord-bot/.env
   # 각 .env 파일 편집하여 실제 credentials 입력
   ```

2. **Song Info Server 실행** (터미널 1):
   ```bash
   cargo run --bin maimai-song-info
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
- 각 서비스는 자신의 `.env`만 로드하여 보안을 강화합니다.
- Song Info Server는 독립적으로 실행 가능하며, Record Collector Server나 Discord 봇 없이도 사용할 수 있습니다.

### Docker로 실행

Docker Compose를 사용하여 세 개의 서비스를 함께 실행할 수 있습니다.

#### 환경 변수 설정

프로젝트 루트에 `.env` 파일을 생성하고 모든 환경 변수를 설정하세요:

```bash
cp .env.example .env
# 편집: SEGA_ID, SEGA_PASSWORD, DISCORD_BOT_TOKEN, DISCORD_USER_ID 등 입력
```

**중요**: Docker Compose는 서비스 이름으로 통신합니다:
- `SONG_INFO_SERVER_URL=http://song-info-server:3001`
- `RECORD_COLLECTOR_SERVER_URL=http://record-collector-server:3000`

#### Docker 빌드 최적화

이 프로젝트는 **단일 Dockerfile**을 사용하여 세 서비스를 모두 빌드합니다:

- **빌더 스테이지**: 전체 워크스페이스를 한 번만 컴파일
- **멀티 타겟**: `target` 옵션으로 각 서비스의 런타임 이미지 생성 (`maimai-song-info`, `record-collector-server`, `personal-discord-bot`)
- **효율성**: 중복 빌드 없이 세 바이너리를 동시에 생성

개별 서비스 빌드:
```bash
docker compose build song-info-server        # song-info-server만 빌드
docker compose build record-collector-server # record-collector-server만 빌드
docker compose build personal-discord-bot     # discord만 빌드
docker compose build                         # 모든 서비스 빌드
```

#### 실행

```bash
docker compose up -d
```

#### 로그 확인

```bash
docker compose logs -f song-info-server
docker compose logs -f record-collector-server
docker compose logs -f personal-discord-bot
```

#### 종료

```bash
docker compose down
```

#### 데이터 영속성

- Song Info Server:
  - 곡 데이터: `./data/song_data/data.json`
  - 재킷 이미지: `./data/song_data/cover/`
- Record Collector Server:
  - SQLite 데이터베이스: `./data/maimai.sqlite3`
  - 쿠키: `./data/cookies.json`
- `docker compose down`을 실행해도 데이터는 유지됩니다.

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
  - 1곡만 매칭해서 보여줍니다.
  - exact match가 없으면 가장 가까운 제목 5개를 버튼으로 제시하고, 선택하면 해당 안내 메시지를 삭제한 뒤 선택한 제목으로 다시 조회합니다.
  - 기록이 없는(미플레이) 항목은 출력하지 않습니다.
- `/mai-recent`
  - recent 페이지 기준 “가장 최근 1 credit”만 보여줍니다.
  - 맨 앞의 `TRACK 01`을 기준으로 그 credit의 플레이들을 `TRACK 01 -> ...` 순서로 출력합니다.

## 데이터 모델/저장 방식 (요약)

- `scores` / `playlogs`에 `achievement_x10000`으로 저장합니다 (`percent * 10000`, 반올림).
- 난이도/차트/랭크/FC/SYNC는 문자열 아이콘을 enum으로 파싱하지만, DB에는 표시용 문자열(TEXT)로 저장합니다.
