import { useCallback, useRef, useState } from 'react';
import type { ReactNode } from 'react';

import {
  checkRecordCollectorHealth,
  formatApiErrorMessage,
  LocalizedApiError,
} from '../api';
import { useI18n } from '../app/i18n';

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
  const { t } = useI18n();
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
      const message = formatApiErrorMessage(error, t);
      setCheckError(
        error instanceof LocalizedApiError && !error.shouldWrap
          ? message
          : t('home.connect.failed', { message }),
      );
    } finally {
      if (!controller.signal.aborted) {
        setIsChecking(false);
      }
    }
  }, [onConnect, t, urlDraft]);

  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">{sidebarTopContent}</aside>

      <div className="table-column home-content">
        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>{t('home.connect.title')}</h2>
              <p>{t('home.connect.description')}</p>
            </div>
          </div>

          <div className="home-connect-row">
            <label className="home-url-field">
              <span>{t('home.connect.serverUrl')}</span>
              <input
                type="url"
                value={urlDraft}
                placeholder={t('home.connect.placeholder')}
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
              {isChecking ? t('common.connecting') : t('common.connect')}
            </button>
          </div>

          {checkError && (
            <p className="home-status home-status-error">{checkError}</p>
          )}
          {connectedPlayer && (
            <div className="home-status home-status-success">
              <span>{t('home.connect.success', { name: connectedPlayer })}</span>
              <button type="button" className="home-goto-btn" onClick={onNavigateToScores}>
                {t('home.connect.goToScores')}
              </button>
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>{t('home.guide.title')}</h2>
              <p>{t('home.guide.description')}</p>
            </div>
          </div>

          <div className="home-steps">
            <div className="home-step">
              <div className="home-step-num">1</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step1Title')}</strong>
                <p>
                  {t('home.guide.step1BodyA')}
                  <code>compose.yaml</code>
                  {t('home.guide.step1BodyB')}
                  <code>SEGA_ID</code>
                  {t('home.guide.step1BodyC')}
                  <code>SEGA_PASSWORD</code>
                  {t('home.guide.step1BodyD')}
                </p>
                <pre className="home-code">{COMPOSE_YAML}</pre>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">2</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step2Title')}</strong>
                <p>
                  {t('home.guide.step2BodyA')}
                  <code>compose.yaml</code>
                  {t('home.guide.step2BodyB')}
                </p>
                <pre className="home-code">docker compose up -d</pre>
                <p>
                  {t('home.guide.step2BodyC')}
                  <code>/health/ready</code>
                  {t('home.guide.step2BodyD')}
                </p>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">3</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step3Title')}</strong>
                <p>{t('home.guide.step3Body')}</p>
              </div>
            </div>

            <div className="home-step">
              <div className="home-step-num">4</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step4Title')}</strong>
                <p>
                  {t('home.guide.step4BodyA')}
                  <strong>{t('common.connect')}</strong>
                  {t('home.guide.step4BodyB')}
                </p>
              </div>
            </div>
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>{t('home.discord.title')}</h2>
              <p>
                {t('home.discord.description')}
                <code>/mai-score</code>, <code>/mai-recent</code>
                {t('home.discord.descriptionTail')}
              </p>
            </div>
          </div>
          <a
            href="https://discord.com/oauth2/authorize?client_id=1463175635974361183"
            target="_blank"
            rel="noreferrer"
            className="home-discord-btn"
          >
            {t('home.discord.addButton')}
          </a>
        </section>

        <footer className="home-footer">
          <ul className="home-footer-credits">
            <li>
              {t('home.footer.aliases')}
              <a href="https://github.com/lomotos10/GCM-bot" target="_blank" rel="noreferrer">
                GCM-bot
              </a>
              {t('home.footer.aliasesTail')}
            </li>
            <li>
              {t('home.footer.constants')}
              <a href="https://x.com/maiLv_Chihooooo" target="_blank" rel="noreferrer">
                maimai譜面定数ちほー
              </a>
              {t('home.footer.constantsTail')}
            </li>
            <li>
              {t('home.footer.parsing')}
              <a href="https://github.com/zetaraku/arcade-songs-fetch" target="_blank" rel="noreferrer">
                arcade-songs-fetch
              </a>
              {t('home.footer.parsingTail')}
            </li>
            <li>
              {t('home.footer.source')}
              <a href="https://github.com/minty99/maistats" target="_blank" rel="noreferrer">
                github.com/minty99/maistats
              </a>
              {t('home.footer.sourceTail')}
            </li>
            <li>
              {t('home.footer.developer')}{' '}
              <a href="https://github.com/minty99" target="_blank" rel="noreferrer">
                github.com/minty99
              </a>
            </li>
          </ul>
          <p className="home-footer-copyright">
            {t('home.footer.copyrightA')}
            <a href="https://maimai.sega.com/" target="_blank" rel="noreferrer">
              maimai DX
            </a>
            {t('home.footer.copyrightB')}
            <a href="https://www.sega.com/" target="_blank" rel="noreferrer">
              SEGA
            </a>{' '}
            {t('home.footer.copyrightC')}
            <a href="https://maimai.sega.com/song/new/#copy--list" target="_blank" rel="noreferrer">
              {t('home.footer.copyrightOwners')}
            </a>
            {t('home.footer.copyrightD')}
          </p>
        </footer>
      </div>
    </div>
  );
}
