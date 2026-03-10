import { useCallback, useRef, useState } from 'react';
import type { Dispatch, ReactNode, SetStateAction } from 'react';

import { checkRecordCollectorHealth } from '../api';

interface SettingsPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrlDraft: string;
  setSongInfoUrlDraft: Dispatch<SetStateAction<string>>;
  recordCollectorUrlDraft: string;
  setRecordCollectorUrlDraft: Dispatch<SetStateAction<string>>;
  onApplySongInfoUrl: () => void;
  onApplyRecordCollectorUrl: (url: string) => void;
}

export function SettingsPage({
  sidebarTopContent,
  songInfoUrlDraft,
  setSongInfoUrlDraft,
  recordCollectorUrlDraft,
  setRecordCollectorUrlDraft,
  onApplySongInfoUrl,
  onApplyRecordCollectorUrl,
}: SettingsPageProps) {
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
      const message = error instanceof Error ? error.message : String(error);
      setRcCheckError(message);
    } finally {
      if (!controller.signal.aborted) {
        setIsRcChecking(false);
      }
    }
  }, [recordCollectorUrlDraft, onApplyRecordCollectorUrl]);

  return (
    <div className="explorer-layout settings-layout">
      <aside className="sidebar-column">{sidebarTopContent}</aside>

      <div className="table-column">
        <section className="panel settings-panel">
          <div className="panel-heading">
            <div>
              <h2>Connections</h2>
              <p>Song Info와 Record Collector 연결 정보를 관리합니다.</p>
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
                적용
              </button>
            </div>
            <p className="settings-warning">
              ⚠ 디버깅 목적이 아니라면 변경하지 마세요.
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
                {isRcChecking ? '연결 중...' : '연결'}
              </button>
            </div>
            {rcCheckError && (
              <p className="home-status home-status-error">연결 실패: {rcCheckError}</p>
            )}
            {rcConnectedPlayer && (
              <div className="home-status home-status-success">
                <span>
                  연결 성공! 플레이어: <strong>{rcConnectedPlayer}</strong>
                </span>
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
