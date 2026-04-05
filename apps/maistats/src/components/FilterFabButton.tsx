interface FilterFabButtonProps {
  label: string;
  onClick: () => void;
}

export function FilterFabButton({ label, onClick }: FilterFabButtonProps) {
  return (
    <button
      type="button"
      className="mobile-filter-fab"
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path
          d="M4 6h16l-6.5 7.5V19l-3 1v-6.5L4 6Z"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinejoin="round"
        />
      </svg>
    </button>
  );
}
