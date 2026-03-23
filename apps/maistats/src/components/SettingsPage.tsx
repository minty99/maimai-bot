import { useCallback, useEffect, useRef, useState } from 'react';
import type { Dispatch, ReactNode, SetStateAction } from 'react';

import {
  checkRecordCollectorHealth,
  fetchCollectorLogs,
  formatApiErrorMessage,
  LocalizedApiError,
} from '../api';
import type { CollectorLogEntry } from '../types';
import { useI18n, type LanguagePreference } from '../app/i18n';

type ThemePreference = 'system' | 'light' | 'dark';

interface SettingsPageProps {
  sidebarTopContent?: ReactNode;
  languagePreference: LanguagePreference;
  setLanguagePreference: (value: LanguagePreference) => void;
  languageLabel: string;
  themePreference: ThemePreference;
  setThemePreference: (value: ThemePreference) => void;
  songInfoUrlDraft: string;
  setSongInfoUrlDraft: Dispatch<SetStateAction<string>>;
  recordCollectorUrlDraft: string;
  setRecordCollectorUrlDraft: Dispatch<SetStateAction<string>>;
  recordCollectorUrl: string;
  onApplySongInfoUrl: () => void;
  onApplyRecordCollectorUrl: (url: string) => void;
}

export function SettingsPage({
  sidebarTopContent,
  languagePreference,
  setLanguagePreference,
  languageLabel,
  themePreference,
  setThemePreference,
  songInfoUrlDraft,
  setSongInfoUrlDraft,
  recordCollectorUrlDraft,
  setRecordCollectorUrlDraft,
  recordCollectorUrl,
  onApplySongInfoUrl,
  onApplyRecordCollectorUrl,
}: SettingsPageProps) {
  const { t } = useI18n();
  const [isRcChecking, setIsRcChecking] = useState(false);
  const [rcCheckError, setRcCheckError] = useState<string | null>(null);
  const [rcConnectedPlayer, setRcConnectedPlayer] = useState<string | null>(null);
  const [collectorLogs, setCollectorLogs] = useState<CollectorLogEntry[]>([]);
  const [collectorLogsError, setCollectorLogsError] = useState<string | null>(null);
  const [isCollectorLogsLoading, setIsCollectorLogsLoading] = useState(false);
  const rcAbortRef = useRef<AbortController | null>(null);
  const logsAbortRef = useRef<AbortController | null>(null);

  const handleSaveLogs = useCallback(() => {
    if (collectorLogs.length === 0) {
      return;
    }

    const blob = new Blob([collectorLogs.map((entry) => entry.line).join('\n')], {
      type: 'text/plain;charset=utf-8',
    });
    const objectUrl = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = objectUrl;
    anchor.download = `maistats-record-collector-logs-${new Date().toISOString().replace(/:/g, '-')}.txt`;
    anchor.click();
    URL.revokeObjectURL(objectUrl);
  }, [collectorLogs]);

  const loadCollectorLogs = useCallback(async (baseUrl?: string) => {
    const url = (baseUrl ?? recordCollectorUrl).trim();
    if (!url) {
      logsAbortRef.current?.abort();
      setCollectorLogs([]);
      setCollectorLogsError(null);
      setIsCollectorLogsLoading(false);
      return;
    }

    logsAbortRef.current?.abort();
    const controller = new AbortController();
    logsAbortRef.current = controller;

    setIsCollectorLogsLoading(true);
    setCollectorLogsError(null);

    try {
      const response = await fetchCollectorLogs(url, controller.signal);
      if (controller.signal.aborted) return;

      setCollectorLogs(response.logs);
    } catch (error) {
      if (controller.signal.aborted) return;
      setCollectorLogsError(
        t('settings.logs.failed', { message: formatApiErrorMessage(error, t) }),
      );
    } finally {
      if (!controller.signal.aborted) {
        setIsCollectorLogsLoading(false);
      }
    }
  }, [recordCollectorUrl, t]);

  useEffect(() => {
    void loadCollectorLogs();

    return () => {
      logsAbortRef.current?.abort();
    };
  }, [loadCollectorLogs]);

  const handleConnectRc = useCallback(async () => {
    const url = recordCollectorUrlDraft.trim();
    if (!url) return;

    rcAbortRef.current?.abort();
    const controller = new AbortController();
    rcAbortRef.current = controller;

    setIsRcChecking(true);
    setRcCheckError(null);
    setRcConnectedPlayer(null);

    try {
      const profile = await checkRecordCollectorHealth(url, controller.signal);
      if (controller.signal.aborted) return;
      setRcConnectedPlayer(profile.user_name);
      onApplyRecordCollectorUrl(url);
      void loadCollectorLogs(url);
    } catch (error) {
      if (controller.signal.aborted) return;
      const message = formatApiErrorMessage(error, t);
      setRcCheckError(
        error instanceof LocalizedApiError && !error.shouldWrap
          ? message
          : t('settings.recordCollector.failed', { message }),
      );
    } finally {
      if (!controller.signal.aborted) {
        setIsRcChecking(false);
      }
    }
  }, [loadCollectorLogs, onApplyRecordCollectorUrl, recordCollectorUrlDraft, t]);

  return (
    <div className="explorer-layout settings-layout">
      <aside className="sidebar-column">{sidebarTopContent}</aside>

      <div className="table-column">
        <section className="panel settings-panel">
          <div className="panel-heading compact">
            <div>
              <h3>{t('settings.title')}</h3>
              <p>{t('settings.description')}</p>
            </div>
          </div>

          <div className="settings-field-group">
            <div className="home-connect-row">
              <label className="home-url-field">
                <span>Song Info URL</span>
                <input
                  type="url"
                  value={songInfoUrlDraft}
                  onChange={(e) => setSongInfoUrlDraft(e.target.value)}
                />
              </label>
              <button
                type="button"
                onClick={onApplySongInfoUrl}
                disabled={!songInfoUrlDraft.trim()}
              >
                {t('common.apply')}
              </button>
            </div>
            <p className="settings-warning">
              {t('settings.songInfoWarning')}
            </p>
          </div>

          <hr className="settings-divider" />

          <div className="settings-field-group">
            <div className="home-connect-row">
              <label className="home-url-field">
                <span>Record Collector URL</span>
                <input
                  type="url"
                  value={recordCollectorUrlDraft}
                  onChange={(e) => {
                    setRecordCollectorUrlDraft(e.target.value);
                    setRcCheckError(null);
                    setRcConnectedPlayer(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') void handleConnectRc();
                  }}
                  disabled={isRcChecking}
                />
              </label>
              <button
                type="button"
                className="home-connect-btn"
                onClick={() => void handleConnectRc()}
                disabled={isRcChecking || !recordCollectorUrlDraft.trim()}
              >
                {isRcChecking ? t('common.connecting') : t('common.connect')}
              </button>
            </div>
            {rcCheckError && (
              <p className="home-status home-status-error">{rcCheckError}</p>
            )}
            {rcConnectedPlayer && (
              <div className="home-status home-status-success">
                <span>{t('settings.recordCollector.success', { name: rcConnectedPlayer })}</span>
              </div>
            )}
          </div>

          <hr className="settings-divider" />

          <div className="settings-field-group">
            <div className="panel-heading compact settings-log-header">
              <div>
                <h3>{t('settings.logs.title')}</h3>
                <p>{t('settings.logs.description')}</p>
              </div>
              <div className="settings-log-actions">
                <button
                  type="button"
                  onClick={handleSaveLogs}
                  disabled={collectorLogs.length === 0}
                >
                  {t('settings.logs.save')}
                </button>
                <button
                  type="button"
                  onClick={() => void loadCollectorLogs()}
                  disabled={isCollectorLogsLoading || !recordCollectorUrl.trim()}
                >
                  {isCollectorLogsLoading ? t('settings.logs.refreshing') : t('settings.logs.refresh')}
                </button>
              </div>
            </div>
            {!recordCollectorUrl.trim() ? (
              <p className="settings-meta">{t('settings.logs.emptyUrl')}</p>
            ) : (
              <p className="settings-meta">
                {t('settings.logs.count', {
                  shown: collectorLogs.length,
                })}
              </p>
            )}
            {collectorLogsError ? (
              <p className="home-status home-status-error">{collectorLogsError}</p>
            ) : null}
            <div className="settings-log-shell" aria-busy={isCollectorLogsLoading}>
              {isCollectorLogsLoading && collectorLogs.length === 0 ? (
                <div className="table-loading-state">{t('settings.logs.refreshing')}</div>
              ) : collectorLogs.length > 0 ? (
                <pre className="settings-log-output">
                  {collectorLogs.map((entry) => entry.line).join('\n')}
                </pre>
              ) : (
                <div className="settings-log-empty">
                  {recordCollectorUrl.trim()
                    ? t('settings.logs.empty')
                    : t('settings.logs.emptyUrl')}
                </div>
              )}
            </div>
          </div>

          <hr className="settings-divider" />

          <div className="settings-field-group">
            <div className="panel-heading compact">
              <div>
                <h3>{t('settings.language.title')}</h3>
                <p>{t('settings.language.description')}</p>
              </div>
            </div>
            <label className="home-url-field">
              <span>{t('settings.language.label')}</span>
              <select
                value={languagePreference}
                onChange={(event) => setLanguagePreference(event.target.value as LanguagePreference)}
              >
                <option value="system">{t('settings.language.optionSystem')}</option>
                <option value="ko">{t('settings.language.optionKo')}</option>
                <option value="en">{t('settings.language.optionEn')}</option>
              </select>
            </label>
            <p className="settings-meta">
              {languagePreference === 'system'
                ? t('settings.language.helperSystem', { language: languageLabel })
                : t('settings.language.helperManual', { language: languageLabel })}
            </p>
          </div>

          <hr className="settings-divider" />

          <div className="settings-field-group">
            <div className="panel-heading compact">
              <div>
                <h3>{t('settings.theme.title')}</h3>
                <p>{t('settings.theme.description')}</p>
              </div>
            </div>
            <label className="home-url-field">
              <span>{t('settings.theme.label')}</span>
              <select
                value={themePreference}
                onChange={(event) => setThemePreference(event.target.value as ThemePreference)}
              >
                <option value="system">{t('settings.theme.optionSystem')}</option>
                <option value="light">{t('settings.theme.optionLight')}</option>
                <option value="dark">{t('settings.theme.optionDark')}</option>
              </select>
            </label>
          </div>

          <hr className="settings-divider" />
        </section>
      </div>
    </div>
  );
}
