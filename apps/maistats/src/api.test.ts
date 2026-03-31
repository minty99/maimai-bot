import { afterEach, describe, expect, it, vi } from 'vitest';

import { VERSION_ORDER } from './app/constants';
import {
  describeRecordCollectorVersionStatus,
  LocalizedApiError,
  buildCoverUrl,
  fetchRecordCollectorVersionStatus,
  fetchAllSongMetadata,
  fetchSongVersions,
  formatApiErrorMessage,
} from './api';
import { APP_VERSION, isMinorOrMoreOutdated, parseSemanticVersion } from './version';

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
    const song = metadata.get(JSON.stringify(['Song A', 'maimai', 'Artist A']));

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
      {
        version_index: VERSION_ORDER.indexOf('PRiSM'),
        version_name: 'PRiSM',
        song_count: 2,
      },
    ]);
  });

});

describe('fetchRecordCollectorVersionStatus', () => {
  it('marks collector versions from an older major release as outdated', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ version: '0.9.5' }), {
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    await expect(
      fetchRecordCollectorVersionStatus('https://collector.example.com'),
    ).resolves.toMatchObject({
      currentVersion: APP_VERSION,
      collectorVersion: '0.9.5',
      isOutdated: true,
      issue: 'version_mismatch',
    });
  });

  it('treats an unreachable version endpoint as outdated', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string) => {
        throw new Error('network down');
      }),
    );

    await expect(
      fetchRecordCollectorVersionStatus('https://collector.example.com'),
    ).resolves.toMatchObject({
      currentVersion: APP_VERSION,
      collectorVersion: null,
      isOutdated: true,
      issue: 'unreachable',
    });
  });

  it('treats malformed collector URLs as outdated', async () => {
    await expect(
      fetchRecordCollectorVersionStatus('not a valid url'),
    ).resolves.toMatchObject({
      currentVersion: APP_VERSION,
      collectorVersion: null,
      isOutdated: true,
      issue: 'unreachable',
    });
  });

  it('marks invalid semantic versions as outdated', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ version: 'not-semver' }), {
          headers: { 'content-type': 'application/json' },
        }),
      ),
    );

    await expect(
      fetchRecordCollectorVersionStatus('https://collector.example.com'),
    ).resolves.toMatchObject({
      currentVersion: APP_VERSION,
      collectorVersion: 'not-semver',
      isOutdated: true,
      issue: 'invalid_response',
    });
  });
});

describe('version helpers', () => {
  it('parses semantic versions with prerelease metadata', () => {
    expect(parseSemanticVersion('1.2.3-beta.1+build.9')).toEqual({
      major: 1,
      minor: 2,
      patch: 3,
    });
  });

  it('compares major and minor versions only', () => {
    expect(isMinorOrMoreOutdated('1.2.3', '1.2.0')).toBe(false);
    expect(isMinorOrMoreOutdated('1.2.3', '1.1.9')).toBe(true);
    expect(isMinorOrMoreOutdated('1.0.0', '1.1.0')).toBe(false);
    expect(isMinorOrMoreOutdated('1.2.3', 'nope')).toBeNull();
  });
});

describe('describeRecordCollectorVersionStatus', () => {
  it('maps version mismatch to the outdated translation key', () => {
    expect(
      describeRecordCollectorVersionStatus({
        currentVersion: APP_VERSION,
        collectorVersion: '0.9.5',
        isOutdated: true,
        issue: 'version_mismatch',
      }),
    ).toEqual({
      translationKey: 'recordCollector.version.outdated',
      variables: {
        currentVersion: APP_VERSION,
        collectorVersion: '0.9.5',
      },
    });
  });

  it('maps invalid versions to the invalid translation key', () => {
    expect(
      describeRecordCollectorVersionStatus({
        currentVersion: APP_VERSION,
        collectorVersion: 'not-semver',
        isOutdated: true,
        issue: 'invalid_response',
      }),
    ).toEqual({
      translationKey: 'recordCollector.version.invalid',
      variables: {
        currentVersion: APP_VERSION,
        collectorVersion: 'not-semver',
      },
    });
  });

  it('maps unreachable collectors to the unreachable translation key', () => {
    expect(
      describeRecordCollectorVersionStatus({
        currentVersion: APP_VERSION,
        collectorVersion: null,
        isOutdated: true,
        issue: 'unreachable',
      }),
    ).toEqual({
      translationKey: 'recordCollector.version.unreachable',
      variables: {
        currentVersion: APP_VERSION,
      },
    });
  });

  it('returns null for compatible or missing statuses', () => {
    expect(describeRecordCollectorVersionStatus(null)).toBeNull();
    expect(
      describeRecordCollectorVersionStatus({
        currentVersion: APP_VERSION,
        collectorVersion: APP_VERSION,
        isOutdated: false,
        issue: null,
      }),
    ).toBeNull();
  });
});
