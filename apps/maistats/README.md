# maistats

`song-info-server`와 `record-collector-server` 데이터를 탐색하는 Vite + React 기반 웹 프론트엔드입니다.

## Requirements

- Node.js 20+ (권장: 20 LTS 이상)
- npm 10+

## What It Does

- 점수 목록과 플레이 로그 탐색
- 곡명, 차트 타입, 난이도, 버전, 랭크, FC, SYNC 기준 필터링
- 달성률, 내부 레벨, 경과일 기준 정렬 및 범위 검색
- 곡별 상세 조회와 chart별 `play_count` 확인
- 배포 환경 변수와 브라우저 UI 설정을 통한 API origin 전환

## Quick Start

1. 의존성 설치:

```bash
npm install
```

2. 환경 변수 파일 생성:

```bash
cp .env.example .env
```

3. 필요하면 `.env` 값을 수정:

```env
SONG_INFO_SERVER_URL=<your-song-info-server-origin>
RECORD_COLLECTOR_SERVER_URL=<your-record-collector-server-origin>
```

로컬 기본값은 [.env.example](./.env.example)에 들어 있습니다.

4. 개발 서버 실행:

```bash
npm run dev
```

접속 가능한 로컬 주소는 Vite가 터미널에 출력합니다.

## Environment Variables

- `SONG_INFO_SERVER_URL`
  - `song-info-server` origin
- `RECORD_COLLECTOR_SERVER_URL`
  - `record-collector-server` origin

이 값들은 앱의 기본 API 연결 주소로 사용됩니다. 실행 중에는 UI의 `Server Connection`에서 브라우저별로 덮어쓸 수 있습니다.

Cloudflare Pages에 배포할 때는 이 값을 저장소에 커밋하지 말고 Pages 환경 변수로 설정하세요.

## Scripts

- `npm run dev`: 개발 서버 실행
- `npm run build`: TypeScript 체크 후 프로덕션 빌드 생성
- `npm run preview`: 빌드 결과 로컬 프리뷰

## Build

```bash
npm run build
```

빌드 결과물은 `dist/`에 생성됩니다.

프리뷰가 필요하면 다음을 실행합니다.

```bash
npm run preview
```

프리뷰 주소는 Vite가 터미널에 출력합니다.

## Deploying With Cloudflare Pages

권장 배포 대상은 Cloudflare Pages입니다.

기본 설정:

- GitHub 저장소 연결
- Production branch: `main`
- Framework preset: `Vite` 또는 `None`
- Build command: `npm ci && npm run build`
- Build output directory: `dist`
- Root directory: `/`
- `NODE_VERSION=20`
- Deploy command: `npx wrangler deploy`

환경 변수:

- Production과 Preview 모두에 `SONG_INFO_SERVER_URL`, `RECORD_COLLECTOR_SERVER_URL` 설정
- 필요하면 custom domain 연결

이 저장소는 `@cloudflare/vite-plugin`과 `wrangler.jsonc`를 포함하므로 `wrangler deploy` 기준 배포도 지원합니다.

운영 방식:

- `main` push 시 production 배포
- PR 생성/업데이트 시 preview 배포

## Data Notes

- Score 화면의 Last Played/Days는 `record-collector-server`의 `/api/scores/rated` (`scores` 테이블 `last_played_at`) 기준입니다.
- Playlog 화면은 `record-collector-server`의 `/api/recent?limit=10000` (`playlogs` 테이블) 기준입니다.
- chart별 `play_count`는 playlog에서 추정하지 않고, `record-collector-server` score API가 내려주는 값을 직접 사용합니다.
- 더 긴 기간 분석이 필요하면 record collector에 추가 API(예: 전체 playlog 조회)가 필요합니다.
