import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  LocalizedApiError,
  buildCoverUrl,
  fetchAllSongMetadata,
  fetchSongVersions,
  formatApiErrorMessage,
} from './api';

afterEach(() => {
  vi.unstubAllGlobals();
});

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

  it('builds static cover URLs', () => {
    expect(buildCoverUrl('https://maimai-charts.muhwan.dev/', 'cover name.png')).toBe(
      'https://maimai-charts.muhwan.dev/cover/cover%20name.png',
    );
  });

  it('parses song metadata from data.json', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            generatedAt: '2026-03-24T00:00:00Z',
            songs: [
              {
                title: 'Song A',
                genre: 'maimai',
                artist: 'Artist A',
                imageName: 'a.png',
                aliases: { en: ['Alias A'] },
                sheets: [
                  {
                    type: 'dx',
                    difficulty: 'master',
                    level: '14',
                    version: 'PRiSM',
                    internalLevel: '14.3',
                    region: { jp: true, intl: true },
                  },
                ],
              },
            ],
          }),
          { headers: { 'content-type': 'application/json' } },
        ),
      ),
    );

    const metadata = await fetchAllSongMetadata('https://maimai-charts.muhwan.dev');
    const song = metadata.get('Song A::maimai::Artist A');

    expect(song).toMatchObject({
      title: 'Song A',
      image_name: 'a.png',
    });
    expect(song?.sheets[0]).toMatchObject({
      chart_type: 'DX',
      difficulty: 'MASTER',
      internal_level: 14.3,
    });
  });

  it('derives version options from data.json', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(
          JSON.stringify({
            generatedAt: '2026-03-24T00:00:00Z',
            songs: [
              {
                title: 'Song A',
                genre: 'maimai',
                artist: 'Artist A',
                sheets: [
                  {
                    type: 'dx',
                    difficulty: 'master',
                    level: '14',
                    version: 'PRiSM',
                    region: { jp: true, intl: true },
                  },
                ],
              },
              {
                title: 'Song B',
                genre: 'maimai',
                artist: 'Artist B',
                sheets: [
                  {
                    type: 'dx',
                    difficulty: 'master',
                    level: '14',
                    version: 'PRiSM',
                    region: { jp: true, intl: true },
                  },
                  {
                    type: 'dx',
                    difficulty: 'expert',
                    level: '13',
                    version: 'PRiSM',
                    region: { jp: true, intl: true },
                  },
                  {
                    type: 'dx',
                    difficulty: 'expert',
                    level: '13',
                    version: 'BUDDiES',
                    region: { jp: true, intl: false },
                  },
                ],
              },
            ],
          }),
          { headers: { 'content-type': 'application/json' } },
        ),
      ),
    );

    await expect(fetchSongVersions('https://maimai-charts.muhwan.dev')).resolves.toEqual([
      { version_index: 24, version_name: 'PRiSM', song_count: 2 },
    ]);
  });
});
