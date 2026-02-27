use serde::{Deserialize, Serialize};

pub const DUPLICATE_CAPABLE_BASE_TITLES: &[&str] = &["Link"];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SongTitle {
    base_title: String,
    qualifier: Option<String>,
}

impl SongTitle {
    pub fn from_parts(base_title: &str, qualifier: Option<&str>) -> Self {
        let base_title = base_title.trim().to_string();
        let qualifier = qualifier
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| normalize_qualifier_for_title(&base_title, v));
        Self {
            base_title,
            qualifier,
        }
    }

    pub fn parse(value: &str) -> Self {
        let trimmed = value.trim();
        if let Some(without_suffix) = trimmed.strip_suffix("]]")
            && let Some((base, qualifier)) = without_suffix.rsplit_once("[[")
        {
            let base = base.trim();
            let qualifier = qualifier.trim();
            if !base.is_empty() && !qualifier.is_empty() {
                return Self::from_parts(base, Some(qualifier));
            }
        }
        Self::from_parts(trimmed, None)
    }

    pub fn base_title(&self) -> &str {
        &self.base_title
    }

    pub fn qualifier(&self) -> Option<&str> {
        self.qualifier.as_deref()
    }

    pub fn requires_qualifier(&self) -> bool {
        Self::requires_qualifier_for(self.base_title())
    }

    pub fn requires_qualifier_for(title: &str) -> bool {
        let candidate = title.trim();
        DUPLICATE_CAPABLE_BASE_TITLES
            .iter()
            .any(|duplicate| duplicate.eq_ignore_ascii_case(candidate))
    }

    pub fn is_ambiguous_unqualified(&self) -> bool {
        self.requires_qualifier() && self.qualifier.is_none()
    }

    pub fn canonical(&self) -> String {
        match self.qualifier() {
            Some(qualifier) => format!("{} [[{}]]", self.base_title(), qualifier),
            None => self.base_title().to_string(),
        }
    }

    pub fn equals_canonical_ignore_ascii_case(&self, other: &SongTitle) -> bool {
        self.canonical()
            .eq_ignore_ascii_case(other.canonical().as_str())
    }
}

fn normalize_qualifier_for_title(base_title: &str, qualifier: &str) -> String {
    let qualifier = qualifier.trim();
    if !SongTitle::requires_qualifier_for(base_title) {
        return qualifier.to_string();
    }

    if base_title.eq_ignore_ascii_case("Link") {
        if matches!(
            qualifier,
            "niconico＆ボーカロイド" | "niconico＆VOCALOID™" | "ORANGE"
        ) {
            return "niconico＆VOCALOID™".to_string();
        }
        if qualifier.eq_ignore_ascii_case("maimai PLUS") || qualifier.eq_ignore_ascii_case("maimai")
        {
            return "maimai".to_string();
        }
    }

    qualifier.to_string()
}

#[cfg(test)]
mod tests {
    use super::SongTitle;

    #[test]
    fn parses_plain_title() {
        let title = SongTitle::parse(" Technicians High ");
        assert_eq!(title.base_title(), "Technicians High");
        assert_eq!(title.qualifier(), None);
        assert_eq!(title.canonical(), "Technicians High");
    }

    #[test]
    fn parses_qualified_title() {
        let title = SongTitle::parse("Link [[niconico＆VOCALOID™]]");
        assert_eq!(title.base_title(), "Link");
        assert_eq!(title.qualifier(), Some("niconico＆VOCALOID™"));
        assert_eq!(title.canonical(), "Link [[niconico＆VOCALOID™]]");
    }

    #[test]
    fn detects_ambiguous_unqualified_title() {
        let title = SongTitle::parse("Link");
        assert!(title.is_ambiguous_unqualified());
    }

    #[test]
    fn normalizes_known_duplicate_qualifier_aliases() {
        assert_eq!(
            SongTitle::from_parts("Link", Some("niconico＆ボーカロイド")).canonical(),
            "Link [[niconico＆VOCALOID™]]"
        );
        assert_eq!(
            SongTitle::from_parts("Link", Some("ORANGE")).canonical(),
            "Link [[niconico＆VOCALOID™]]"
        );
        assert_eq!(
            SongTitle::from_parts("Link", Some("maimai PLUS")).canonical(),
            "Link [[maimai]]"
        );
    }
}
