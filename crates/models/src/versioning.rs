use eyre::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionApiResponse {
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SemanticVersion {
    major: u64,
    minor: u64,
    _patch: u64,
}

impl SemanticVersion {
    fn is_minor_or_more_outdated_than(self, current: Self) -> bool {
        self.major < current.major || (self.major == current.major && self.minor < current.minor)
    }
}

impl FromStr for SemanticVersion {
    type Err = eyre::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        ensure!(!trimmed.is_empty(), "version must not be empty");

        let core = &trimmed[..trimmed.find(['-', '+']).unwrap_or(trimmed.len())];
        let mut parts = core.split('.');

        let major = parts
            .next()
            .ok_or_else(|| eyre::eyre!("missing major version"))?
            .parse::<u64>()?;
        let minor = parts
            .next()
            .ok_or_else(|| eyre::eyre!("missing minor version"))?
            .parse::<u64>()?;
        let patch = parts
            .next()
            .ok_or_else(|| eyre::eyre!("missing patch version"))?
            .parse::<u64>()?;

        ensure!(
            parts.next().is_none(),
            "version must contain exactly three numeric components"
        );

        Ok(Self {
            major,
            minor,
            _patch: patch,
        })
    }
}

pub fn is_minor_or_more_outdated(current_version: &str, candidate_version: &str) -> Result<bool> {
    let current = current_version.parse::<SemanticVersion>()?;
    let candidate = candidate_version.parse::<SemanticVersion>()?;

    Ok(candidate.is_minor_or_more_outdated_than(current))
}

#[cfg(test)]
mod tests {
    use super::is_minor_or_more_outdated;

    #[test]
    fn parses_semver_with_prerelease_and_build_metadata() {
        assert!(
            !is_minor_or_more_outdated("1.2.0", "1.2.3-alpha.1+build.5")
                .expect("version comparison should succeed")
        );
    }

    #[test]
    fn treats_lower_minor_versions_as_outdated() {
        assert!(
            is_minor_or_more_outdated("1.2.0", "1.1.9").expect("version comparison should succeed")
        );
    }

    #[test]
    fn ignores_patch_differences_within_the_same_minor() {
        assert!(
            !is_minor_or_more_outdated("1.2.3", "1.2.0")
                .expect("version comparison should succeed")
        );
    }

    #[test]
    fn treats_lower_major_versions_as_outdated() {
        assert!(
            is_minor_or_more_outdated("2.0.0", "1.9.9").expect("version comparison should succeed")
        );
    }

    #[test]
    fn rejects_invalid_versions() {
        assert!(is_minor_or_more_outdated("1.0.0", "1.0").is_err());
        assert!(is_minor_or_more_outdated("1.0.0", "latest").is_err());
    }
}
