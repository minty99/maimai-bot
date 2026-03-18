import { describe, expect, it, vi } from 'vitest';

import { LocalizedApiError, formatApiErrorMessage } from './api';

describe('formatApiErrorMessage', () => {
  it('formats localized API errors with the provided translator', () => {
    const t = vi.fn((key: string, variables?: Record<string, string | number>) =>
      `${key}:${variables?.status ?? 'none'}`,
    );

    expect(
      formatApiErrorMessage(new LocalizedApiError('api.connectionFailed', { status: 503 }), t),
    ).toBe('api.connectionFailed:503');
    expect(t).toHaveBeenCalledWith('api.connectionFailed', { status: 503 });
  });

  it('passes through generic error messages', () => {
    expect(formatApiErrorMessage(new Error('network down'), vi.fn())).toBe('network down');
  });
});
