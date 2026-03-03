import type { Dispatch, SetStateAction } from 'react';

interface ServerConnectionModalProps {
  isOpen: boolean;
  songInfoUrl: string;
  recordCollectorUrl: string;
  songInfoUrlDraft: string;
  setSongInfoUrlDraft: Dispatch<SetStateAction<string>>;
  recordCollectorUrlDraft: string;
  setRecordCollectorUrlDraft: Dispatch<SetStateAction<string>>;
  onClose: () => void;
  onApply: () => void;
}

export function ServerConnectionModal({
  isOpen,
  songInfoUrl,
  recordCollectorUrl,
  songInfoUrlDraft,
  setSongInfoUrlDraft,
  recordCollectorUrlDraft,
  setRecordCollectorUrlDraft,
  onClose,
  onApply,
}: ServerConnectionModalProps) {
  if (!isOpen) {
    return null;
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <section className="modal-card panel server-modal" onClick={(event) => event.stopPropagation()}>
        <div className="detail-header">
          <h2>Server Connection</h2>
          <button type="button" onClick={onClose}>
            닫기
          </button>
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
  );
}
