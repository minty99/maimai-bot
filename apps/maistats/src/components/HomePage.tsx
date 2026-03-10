import { useCallback, useRef, useState } from 'react';
import type { ReactNode } from 'react';

import { checkRecordCollectorHealth } from '../api';

const COMPOSE_YAML = `name: maistats-record-collector

services:
  maistats-record-collector:
    image: ghcr.io/minty99/maistats-record-collector:latest
    container_name: maistats-record-collector
    ports:
      - "3002:3000"
    environment:
      SEGA_ID: \${SEGA_ID}
      SEGA_PASSWORD: \${SEGA_PASSWORD}
      RECORD_COLLECTOR_PORT: "3000"
      DATA_DIR: /app/data
      DATABASE_URL: sqlite:/app/data/records.sqlite3
      RUST_LOG: \${RUST_LOG:-info}
    volumes:
      - ./data:/app/data
    restart: unless-stopped`;

interface HomePageProps {
  sidebarTopContent?: ReactNode;
  recordCollectorUrl: string;
  onConnect: (url: string) => void;
  onNavigateToScores: () => void;
}

export function HomePage({
  sidebarTopContent,
  recordCollectorUrl,
  onConnect,
  onNavigateToScores,
}: HomePageProps) {
  const [urlDraft, setUrlDraft] = useState(recordCollectorUrl || '');
  const [isChecking, setIsChecking] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);
  const [connectedPlayer, setConnectedPlayer] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const handleConnect = useCallback(async () => {
    const url = urlDraft.trim();
    if (!url) return;

    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setIsChecking(true);
    setCheckError(null);
    setConnectedPlayer(null);

    try {
      const profile = await checkRecordCollectorHealth(url, controller.signal);
      if (controller.signal.aborted) return;
      setConnectedPlayer(profile.user_name);
      onConnect(url);
    } catch (error) {
      if (controller.signal.aborted) return;
      const message = error instanceof Error ? error.message : String(error);
      setCheckError(message);
    } finally {
      if (!controller.signal.aborted) {
        setIsChecking(false);
      }
    }
  }, [urlDraft, onConnect]);

  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">{sidebarTopContent}</aside>

      <div className="table-column home-content">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>Record Collector 연결</h2>
              <p>Record Collector 서버 URL을 입력하고 연결을 확인합니다.</p>
            </div>
          </div>

          <div className="home-connect-row">
            <label className="home-url-field">
              <span>서버 URL</span>
              <input
                type="url"
                value={urlDraft}
                placeholder="https://your-server.example.com"
                onChange={(e) => setUrlDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void handleConnect();
                }}
                disabled={isChecking}
              />
            </label>
            <button
              type="button"
              className="home-connect-btn"
              onClick={() => void handleConnect()}
              disabled={isChecking || !urlDraft.trim()}
            >
              {isChecking ? '연결 중...' : '연결'}
            </button>
          </div>

          {checkError && (
            <p className="home-status home-status-error">연결 실패: {checkError}</p>
          )}
          {connectedPlayer && (
            <div className="home-status home-status-success">
              <span>
                연결 성공! 플레이어: <strong>{connectedPlayer}</strong>
              </span>
              <button type="button" className="home-goto-btn" onClick={onNavigateToScores}>
                Scores로 이동 →
              </button>
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>서버 실행 가이드</h2>
              <p>Record Collector 서버가 없다면 아래 안내를 따라 직접 실행하세요.</p>
            </div>
          </div>

          <div className="home-steps">
            <div className="home-step">
              <div className="home-step-num">1</div>
              <div className="home-step-body">
                <strong>compose.yaml 파일 생성</strong>
                <p>
                  서버를 실행할 폴더에 아래 내용으로 <code>compose.yaml</code> 파일을 만듭니다.{' '}
                  <code>SEGA_ID</code>와 <code>SEGA_PASSWORD</code>에 maimaidx-eng.com 계정
                  정보를 입력하세요.
                </p>
                <pre className="home-code">{COMPOSE_YAML}</pre>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">2</div>
              <div className="home-step-body">
                <strong>Docker Compose 실행</strong>
                <p>
                  Docker가 설치된 환경에서 <code>compose.yaml</code>이 있는 폴더에서 아래
                  명령어로 컨테이너를 시작합니다.
                </p>
                <pre className="home-code">docker compose up -d</pre>
                <p>
                  첫 실행 시 이미지 다운로드 후 maimaidx-eng.com 로그인이 진행됩니다. 서버가
                  준비되면 <code>/health/ready</code> 엔드포인트가 200을 반환합니다.
                </p>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">3</div>
              <div className="home-step-body">
                <strong>외부 접근 설정 (선택)</strong>
                <p>
                  외부에서 접근하려면 서버를 공개 IP 또는 도메인으로 노출하고 해당 주소를
                  입력하세요. ngrok, Cloudflare Tunnel 등을 활용할 수 있습니다.
                </p>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">4</div>
              <div className="home-step-body">
                <strong>URL 연결</strong>
                <p>
                  서버가 준비되면 위 입력창에 서버 URL을 입력하고 <strong>연결</strong> 버튼을
                  클릭하세요. 연결에 성공하면 자동으로 Scores 페이지로 이동합니다.
                </p>
              </div>
            </div>
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>Discord Bot</h2>
              <p>
                Discord 서버에 maistats 봇을 추가하면 <code>/mai-score</code>,{' '}
                <code>/mai-recent</code> 명령어로 스코어와 최근 플레이 기록을 바로 조회할 수
                있습니다.
              </p>
            </div>
          </div>
          <a
            href="https://discord.com/oauth2/authorize?client_id=1463175635974361183"
            target="_blank"
            rel="noreferrer"
            className="home-discord-btn"
          >
            Discord Bot 추가하기
          </a>
        </section>

        <footer className="home-footer">
          <ul className="home-footer-credits">
            <li>
              곡 제목의 alias는{' '}
              <a href="https://github.com/lomotos10/GCM-bot" target="_blank" rel="noreferrer">
                GCM-bot
              </a>
              으로부터 허가를 받아 가져왔습니다.
            </li>
            <li>
              곡들의 보면상수는{' '}
              <a href="https://x.com/maiLv_Chihooooo" target="_blank" rel="noreferrer">
                maimai譜面定数ちほー
              </a>
              에서 가져왔습니다.
            </li>
            <li>
              곡의 파싱은{' '}
              <a href="https://github.com/zetaraku/arcade-songs-fetch" target="_blank" rel="noreferrer">
                arcade-songs-fetch
              </a>
              를 참고했습니다.
            </li>
            <li>
              maistats의 소스 코드는{' '}
              <a href="https://github.com/minty99/maistats" target="_blank" rel="noreferrer">
                github.com/minty99/maistats
              </a>
              에 공개되어 있습니다.
            </li>
            <li>
              개발자:{' '}
              <a href="https://github.com/minty99" target="_blank" rel="noreferrer">
                github.com/minty99
              </a>
            </li>
          </ul>
          <p className="home-footer-copyright">
            본 사이트는 개인 성과 기록 및 추적을 위해 만든{' '}
            <a href="https://maimai.sega.com/" target="_blank" rel="noreferrer">
              maimai DX
            </a>
            의 팬 사이트이며, 사이트 내에 사용된 게임 관련 컨텐츠의 저작권은{' '}
            <a href="https://www.sega.com/" target="_blank" rel="noreferrer">
              SEGA
            </a>{' '}
            및{' '}
            <a href="https://maimai.sega.com/song/new/#copy--list" target="_blank" rel="noreferrer">
              각 소유자들
            </a>
            에게 있습니다.
          </p>
        </footer>
      </div>
    </div>
  );
}
