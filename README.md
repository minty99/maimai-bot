# maimai-bot

`maimai-bot`은 **maimai DX NET (maimaidx-eng.com)** 에 로그인(SEGA ID)해서 기록을 크롤링하고, 로컬 SQLite에 저장한 뒤 Discord에서 조회/알림을 받는 **단일 사용자** Rust 앱입니다.

## 아키텍처

이 프로젝트는 **백엔드(HTTP API)** + **Discord 봇** 구조로 분리되어 있습니다:

- **Backend** (`backend/`): maimai 크롤링, DB 관리, REST API 제공
  - 쿠키를 `data/` 아래에 저장/재사용하고, 만료 시 재로그인해서 갱신합니다.
  - DB는 `sqlx::migrate!()`로 런타임에 마이그레이션을 실행합니다.
  - 시작 시 `playerData`를 크롤링하고, 필요하면 scores(난이도 0..4) + recent를 DB에 초기 적재합니다.
  - 이후 10분마다 `playerData`를 다시 크롤링해서 **total play count 변화가 있을 때만** recent를 크롤링합니다.
  
- **Discord Bot** (`discord/`): 백엔드 API를 호출하여 Discord 명령어 처리 및 DM 알림 전송
  - 백엔드의 `/health/ready` 엔드포인트를 폴링하여 백엔드가 준비될 때까지 대기합니다.
  - 백엔드에서 새 플레이가 감지되면 DM으로 알림을 보냅니다.

**중요**: 백엔드를 먼저 실행한 후 Discord 봇을 실행해야 합니다.

## 특징

- 쿠키 기반 인증으로 SEGA ID 로그인 유지
- SQLite 기반 로컬 데이터 저장
- 자동 스코어 동기화 및 플레이 로그 추적
- Discord 슬래시 커맨드 및 DM 알림

## 요구사항

- Rust (stable)
- Discord Bot Token / 단일 수신자(User ID)
- SEGA ID 계정 (maimaidx-eng.com)

## 설정

환경 변수는 **백엔드**와 **Discord 봇**에 각각 분리되어 있습니다.

### Backend 설정 (`backend/.env`)

`.env.example`을 복사하여 `backend/.env`를 생성하고 다음 값을 설정하세요:

```env
SEGA_ID=your_sega_id_here
SEGA_PASSWORD=your_password_here
PORT=3000
DATABASE_URL=sqlite:../data/maimai.sqlite3
```

### Discord Bot 설정 (`discord/.env`)

`.env.example`을 복사하여 `discord/.env`를 생성하고 다음 값을 설정하세요:

```env
DISCORD_BOT_TOKEN=your_bot_token_here
DISCORD_USER_ID=your_user_id_here
BACKEND_URL=http://localhost:3000
```

**주의**: `.env` 파일은 절대 커밋하지 마세요. `.gitignore`에 포함되어 있습니다.

### 기본 런타임 경로

- DB: `data/maimai.sqlite3`
- 쿠키: `data/cookies.json`

## 실행

### 배포 순서 (중요)

**반드시 백엔드를 먼저 실행한 후 Discord 봇을 실행하세요.**

1. **백엔드 실행**:
   ```bash
   cd backend
   cargo run --bin maimai-backend
   ```
   백엔드는 `http://localhost:3000`에서 실행되며, `/health/ready` 엔드포인트를 제공합니다.

2. **Discord 봇 실행** (별도 터미널):
   ```bash
   cd discord
   cargo run --bin maimai-discord
   ```
   Discord 봇은 백엔드의 `/health/ready`를 폴링하여 백엔드가 준비될 때까지 대기합니다.

### Docker로 실행

Docker Compose를 사용하여 백엔드와 Discord 봇을 함께 실행할 수 있습니다.

#### 환경 변수 설정

프로젝트 루트에 `.env` 파일을 생성하고 다음 값을 설정하세요:

```env
SEGA_ID=your_sega_id_here
SEGA_PASSWORD=your_password_here
DISCORD_BOT_TOKEN=your_bot_token_here
DISCORD_USER_ID=your_user_id_here
```

#### 실행

```bash
docker compose up -d
```

#### 로그 확인

```bash
docker compose logs -f backend
docker compose logs -f discord
```

#### 종료

```bash
docker compose down
```

#### 데이터 영속성

- SQLite 데이터베이스는 `./data/maimai.sqlite3`에 저장됩니다.
- 쿠키는 `./data/cookies.json`에 저장됩니다.
- `docker compose down`을 실행해도 `./data/` 디렉토리의 데이터는 유지됩니다.

### 개발/디버깅 명령어

백엔드에서 제공하는 CLI 명령어들 (레거시, 참고용):

쿠키 로그인/체크:
- `cargo run -- auth login`
- `cargo run -- auth check`

HTML/raw fetch (로그인 필요):
- `cargo run -- fetch url --url https://maimaidx-eng.com/maimai-mobile/playerData/ --out data/out/player_data.html`

크롤링/파싱(JSON)만 수행 (DB 미사용):
- `cargo run -- crawl player-data --out data/out/player_data.json`
- `cargo run -- crawl recent --out data/out/recent.json`
- `cargo run -- crawl scores --out data/out/scores.json`

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
