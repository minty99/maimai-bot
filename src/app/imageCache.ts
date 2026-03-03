const COVER_CACHE_NAME = 'maistats-cover-v1';

const coverObjectUrlCache = new Map<string, string>();
const coverRequestCache = new Map<string, Promise<string>>();

async function readCoverObjectUrl(url: string): Promise<string> {
  if (typeof window === 'undefined' || typeof window.caches === 'undefined') {
    return url;
  }

  const cache = await window.caches.open(COVER_CACHE_NAME);
  let response = await cache.match(url);

  if (!response) {
    response = await fetch(url, {
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch cover image: ${response.status}`);
    }

    await cache.put(url, response.clone());
  }

  const blob = await response.blob();
  const objectUrl = URL.createObjectURL(blob);
  coverObjectUrlCache.set(url, objectUrl);
  return objectUrl;
}

export async function getCachedCoverObjectUrl(url: string): Promise<string> {
  const cached = coverObjectUrlCache.get(url);
  if (cached) {
    return cached;
  }

  const inflight = coverRequestCache.get(url);
  if (inflight) {
    return inflight;
  }

  const request = readCoverObjectUrl(url)
    .catch(() => url)
    .finally(() => {
      coverRequestCache.delete(url);
    });

  coverRequestCache.set(url, request);
  return request;
}
