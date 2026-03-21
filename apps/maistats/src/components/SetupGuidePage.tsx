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

interface SetupGuidePageProps {
  sidebarTopContent?: ReactNode;
  recordCollectorUrl: string;
  onConnect: (url: string) => void;
  onNavigateToScores: () => void;
}

export function SetupGuidePage({
  sidebarTopContent,
  recordCollectorUrl,
  onConnect,
  onNavigateToScores,
}: SetupGuidePageProps) {
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
                onChange={(event) => setUrlDraft(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') void handleConnect();
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

          {checkError ? <p className="home-status home-status-error">{checkError}</p> : null}
          {connectedPlayer ? (
            <div className="home-status home-status-success">
              <span>{t('home.connect.success', { name: connectedPlayer })}</span>
              <button type="button" className="home-goto-btn" onClick={onNavigateToScores}>
                {t('home.connect.goToScores')}
              </button>
            </div>
          ) : null}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <div>
              <h2>{t('home.guide.title')}</h2>
              <p>{t('home.guide.description')}</p>
            </div>
          </div>

          <div className="home-steps">
            <article className="home-step">
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
            </article>

            <article className="home-step">
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
            </article>

            <article className="home-step">
              <div className="home-step-num">3</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step3Title')}</strong>
                <p>{t('home.guide.step3Body')}</p>
              </div>
            </article>

            <article className="home-step">
              <div className="home-step-num">4</div>
              <div className="home-step-body">
                <strong>{t('home.guide.step4Title')}</strong>
                <p>
                  {t('home.guide.step4BodyA')}
                  <strong>{t('common.connect')}</strong>
                  {t('home.guide.step4BodyB')}
                </p>
              </div>
            </article>
          </div>
        </section>
      </div>
    </div>
  );
}
