import { useEffect, useRef, useState } from 'react';

import { buildCoverUrl } from '../api';
import { getCachedCoverObjectUrl } from '../app/imageCache';

interface JacketProps {
  songInfoUrl: string;
  imageName: string | null;
  title: string;
}

export function Jacket({ songInfoUrl, imageName, title }: JacketProps) {
  const coverUrl = imageName ? buildCoverUrl(songInfoUrl, imageName) : null;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [shouldLoad, setShouldLoad] = useState(() => coverUrl === null);
  const [resolvedSrc, setResolvedSrc] = useState<string | null>(null);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    setShouldLoad(coverUrl === null);
    setResolvedSrc(null);
    setHasError(false);
  }, [coverUrl]);

  useEffect(() => {
    if (!coverUrl) {
      return;
    }

    const element = containerRef.current;
    if (!element) {
      return;
    }

    if (typeof window === 'undefined' || !('IntersectionObserver' in window)) {
      setShouldLoad(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) {
          return;
        }
        setShouldLoad(true);
        observer.disconnect();
      },
      { rootMargin: '240px' },
    );

    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [coverUrl]);

  useEffect(() => {
    if (!coverUrl || !shouldLoad) {
      return;
    }

    let cancelled = false;

    void getCachedCoverObjectUrl(coverUrl).then((src) => {
      if (!cancelled) {
        setResolvedSrc(src);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [coverUrl, shouldLoad]);

  if (!imageName || hasError) {
    return <div className="jacket fallback">{title.slice(0, 1).toUpperCase()}</div>;
  }

  return (
    <div ref={containerRef}>
      {resolvedSrc ? (
        <img
          className="jacket"
          src={resolvedSrc}
          alt={`${title} jacket`}
          referrerPolicy="no-referrer"
          onError={() => setHasError(true)}
        />
      ) : (
        <div className="jacket fallback">{title.slice(0, 1).toUpperCase()}</div>
      )}
    </div>
  );
}
