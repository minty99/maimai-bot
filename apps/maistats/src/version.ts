import packageJson from '../package.json';

export const APP_VERSION = packageJson.version;

type SemanticVersion = {
  major: number;
  minor: number;
  patch: number;
};

const SEMVER_RE =
  /^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

export function parseSemanticVersion(value: string): SemanticVersion | null {
  const match = SEMVER_RE.exec(value.trim());
  if (!match) {
    return null;
  }

  const [, majorText, minorText, patchText] = match;
  return {
    major: Number(majorText),
    minor: Number(minorText),
    patch: Number(patchText),
  };
}

export function isMinorOrMoreOutdated(
  currentVersion: string,
  candidateVersion: string,
): boolean | null {
  const current = parseSemanticVersion(currentVersion);
  const candidate = parseSemanticVersion(candidateVersion);

  if (!current || !candidate) {
    return null;
  }

  if (candidate.major !== current.major) {
    return candidate.major < current.major;
  }

  return candidate.minor < current.minor;
}
