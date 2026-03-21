import { useCallback, useRef, useState } from 'react';
import type { Dispatch, ReactNode, SetStateAction } from 'react';

import {
  checkRecordCollectorHealth,
  formatApiErrorMessage,
  LocalizedApiError,
} from '../api';
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
  onApplySongInfoUrl,
  onApplyRecordCollectorUrl,
}: SettingsPageProps) {
  const { t } = useI18n();
  const [isRcChecking, setIsRcChecking] = useState(false);
  const [rcCheckError, setRcCheckError] = useState<string | null>(null);
  const [rcConnectedPlayer, setRcConnectedPlayer] = useState<string | null>(null);
  const rcAbortRef = useRef<AbortController | null>(null);

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
  }, [onApplyRecordCollectorUrl, recordCollectorUrlDraft, t]);

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
                <h3>Theme</h3>
                <p>앱의 색상 테마를 선택합니다.</p>
              </div>
            </div>
            <label className="home-url-field">
              <span>Color theme</span>
              <select
                value={themePreference}
                onChange={(event) => setThemePreference(event.target.value as ThemePreference)}
              >
                <option value="system">System default</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
            </label>
          </div>

          <hr className="settings-divider" />

        </section>
      </div>
    </div>
  );
}
