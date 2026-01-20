# maimai-bot

maimai-bot은 **maimai DX NET**(maimaidx-eng.com)에서 개인 플레이 기록을 주기적으로 수집하고, Discord에서 조회/알림을 받을 수 있게 해주는 단일 사용자용 Rust 애플리케이션입니다.

Discord 봇은 로컬 SQLite DB에 데이터를 저장하며, 10분마다 최근 기록을 확인해 새로운 플레이 기록이 감지되면 지정된 사용자에게 DM으로 요약을 전송합니다.

## Discord 봇 동작

- **주기적 갱신**: 10분마다 최근 플레이 기록을 가져와 DB를 업데이트합니다.
- **새 기록 감지**: `playlog_idx` 기준으로 DB에 없던 기록만 “새 기록”으로 판단합니다.
- **DM 알림**: 새 기록이 있으면 `DISCORD_USER_ID`로 지정된 사용자에게만 DM을 보냅니다.

## Discord에서 사용하는 방법

슬래시 명령어로 사용합니다: `/mai-record`, `/mai-recent`

### 자동 알림 (DM)

새 기록이 발견되면 DM으로 아래 형식의 메시지가 전송됩니다.

```
🎵 **New Records Detected!**

**곡 제목** [STD|DX] 난이도 - 플레이 일시
📊 달성률  🏆 등급  🎯 FC  👥 SYNC  💫 DX 점수/최대 점수
```

### `/mai-record <query>`: 곡별 기록 조회

곡 제목(부분 일치) 또는 `song_key`로 점수 기록을 조회합니다.

예시:

```
/mai-record GALAXY
```

출력은 대략 아래처럼 나옵니다(표시 형식은 서버/클라이언트에 따라 달라질 수 있음).

```
📊 Records for 'GALAXY'

**GALAXY** [STD] BASIC: 100.50% - SSS+
**GALAXY** [DX] MASTER: 99.80% - SSS
```

### `/mai-recent`: 최근 크레딧 기록 조회

DB에 저장된 최근 플레이 로그를 조회합니다.
- `limit` 최대값: 10

예시:

```
/mai-recent
```

```
`/mai-recent`는 가장 최근 1 크레딧(최근 TRACK 01부터)의 플레이만 보여줘요.
```

출력 예시:

```
🕐 Recent 1 Credit (4 plays)

**곡 제목** [STD] - Track 1 @ 2026/01/20 22:10
📊 99.50% - SSS
```

## Discord 설정/주의사항

- 봇을 서버에 초대하고, 메시지 읽기/전송 권한이 필요합니다.
- DM 알림을 받으려면 봇이 사용자에게 DM을 보낼 수 있어야 합니다(서버 설정/사용자 설정에 따라 DM이 차단될 수 있음).
- 이 프로젝트는 **단일 사용자** 전용입니다. DM 알림은 `DISCORD_USER_ID`로 지정된 사용자에게만 전송됩니다.
