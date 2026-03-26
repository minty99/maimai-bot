import { useEffect, useState } from 'react';

import { useI18n } from '../app/i18n';

interface SearchInputProps {
  label: string;
  placeholder: string;
  appliedQuery: string;
  onApplyQuery: (query: string) => void;
}

export function SearchInput({ label, placeholder, appliedQuery, onApplyQuery }: SearchInputProps) {
  const { t } = useI18n();
  const [draft, setDraft] = useState(appliedQuery);

  useEffect(() => {
    setDraft(appliedQuery);
  }, [appliedQuery]);

  const trimmedDraft = draft.trim();

  return (
    <form
      className="search-box search-submit-group filter-block"
      onSubmit={(event) => {
        event.preventDefault();
        onApplyQuery(trimmedDraft);
      }}
    >
      <span>{label}</span>
      <div className="search-submit-row">
        <input
          type="search"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder={placeholder}
        />
        <button type="submit" className="search-submit-button" disabled={trimmedDraft === appliedQuery}>
          {t('common.search')}
        </button>
      </div>
    </form>
  );
}
