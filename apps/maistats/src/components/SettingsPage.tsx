import type { Dispatch, ReactNode, SetStateAction } from 'react';

interface SettingsPageProps {
  sidebarTopContent?: ReactNode;
  songInfoUrl: string;
  recordCollectorUrl: string;
  songInfoUrlDraft: string;
  setSongInfoUrlDraft: Dispatch<SetStateAction<string>>;
  recordCollectorUrlDraft: string;
  setRecordCollectorUrlDraft: Dispatch<SetStateAction<string>>;
  onApply: () => void;
}

export function SettingsPage({
  sidebarTopContent,
  songInfoUrl,
  recordCollectorUrl,
  songInfoUrlDraft,
  setSongInfoUrlDraft,
  recordCollectorUrlDraft,
  setRecordCollectorUrlDraft,
  onApply,
}: SettingsPageProps) {
  return (
    <div className="explorer-layout settings-layout">
      <aside className="sidebar-column">
        {sidebarTopContent}
      </aside>

      <div className="table-column">
        <section className="panel settings-panel">
          <div className="panel-heading">
            <div>
              <h2>Connections</h2>
              <p>Song Info와 Record Collector 연결 정보를 관리합니다.</p>
            </div>
          </div>

          <div className="server-grid">
            <label>
              <span>Song Info URL</span>
              <input
                type="url"
                value={songInfoUrlDraft}
                onChange={(event) => setSongInfoUrlDraft(event.target.value)}
              />
            </label>
            <label>
              <span>Record Collector URL</span>
              <input
                type="url"
                value={recordCollectorUrlDraft}
                onChange={(event) => setRecordCollectorUrlDraft(event.target.value)}
              />
            </label>
            <button type="button" onClick={onApply}>
              URL 적용
            </button>
          </div>

          <div className="server-meta">
            <span>현재 Song Info: {songInfoUrl}</span>
            <span>현재 Record Collector: {recordCollectorUrl || '미설정 (Random Picker에서는 선택 사항)'}</span>
          </div>
        </section>
      </div>
    </div>
  );
}
