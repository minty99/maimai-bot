# maimai-bot

`maimai-bot`은 **maimai DX NET (maimaidx-eng.com)** 에 로그인(SEGA ID)해서 기록을 크롤링하고, 로컬 SQLite에 저장한 뒤 Discord에서 조회/알림을 받는 **단일 사용자** Rust 앱입니다.

## 특징

- 쿠키를 `data/` 아래에 저장/재사용하고, 만료 시 재로그인해서 갱신합니다.
- DB는 `sqlx::migrate!()`로 런타임에 마이그레이션을 실행합니다.
- 봇은 시작 시 `playerData`를 크롤링하고, 필요하면 scores(난이도 0..4) + recent를 DB에 초기 적재합니다.
- 이후 10분마다 `playerData`를 다시 크롤링해서 **total play count 변화가 있을 때만** recent를 크롤링/DM 알림을 보냅니다.

## 요구사항

- Rust (stable)
- Discord Bot Token / 단일 수신자(User ID)
- SEGA ID 계정 (maimaidx-eng.com)

## 설정

`.env` (커밋 금지):

- `SEGA_ID`
- `SEGA_PASSWORD`
- `DISCORD_BOT_TOKEN`
- `DISCORD_USER_ID` (DM을 받을 Discord 유저 ID)

기본 런타임 경로:

- DB: `data/maimai.sqlite3`
- 쿠키: `data/cookies.json`

## 실행

Discord 봇 실행:

- `cargo run -- bot run`

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
