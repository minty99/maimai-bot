pub(crate) fn is_ap_like(fc: Option<&str>) -> bool {
    matches!(fc, Some("AP") | Some("AP+"))
}

pub(crate) fn coefficient_for_achievement(achievement_percent: f64) -> f64 {
    const ACHIEVEMENT_CAP: f64 = 100.5;
    let a = achievement_percent.min(ACHIEVEMENT_CAP);

    if a >= 100.5 {
        22.4
    } else if a >= 100.4999 {
        22.2
    } else if a >= 100.0 {
        21.6
    } else if a >= 99.9999 {
        21.4
    } else if a >= 99.5 {
        21.1
    } else if a >= 99.0 {
        20.8
    } else if a >= 98.9999 {
        20.6
    } else if a >= 98.0 {
        20.3
    } else if a >= 97.0 {
        20.0
    } else if a >= 96.9999 {
        17.6
    } else if a >= 94.0 {
        16.8
    } else if a >= 90.0 {
        15.2
    } else if a >= 80.0 {
        13.6
    } else if a >= 79.9999 {
        12.8
    } else if a >= 75.0 {
        12.0
    } else if a >= 70.0 {
        11.2
    } else if a >= 60.0 {
        9.6
    } else if a >= 50.0 {
        8.0
    } else if a >= 40.0 {
        6.4
    } else if a >= 30.0 {
        4.8
    } else if a >= 20.0 {
        3.2
    } else if a >= 10.0 {
        1.6
    } else {
        0.0
    }
}

pub(crate) fn chart_rating_points(
    internal_level: f64,
    achievement_percent: f64,
    ap_bonus: bool,
) -> u32 {
    const ACHIEVEMENT_CAP: f64 = 100.5;
    let coef = coefficient_for_achievement(achievement_percent);
    let ach = achievement_percent.min(ACHIEVEMENT_CAP);
    let base = ((coef * internal_level * ach) / 100.0).floor();
    let base = if base.is_finite() && base > 0.0 {
        base as u32
    } else {
        0
    };
    if ap_bonus {
        base.saturating_add(1)
    } else {
        base
    }
}

/// Derive a fallback internal level from the displayed level string.
///
/// - If level ends with "+": numeric part + 0.6 (e.g., "13+" → 13.6)
/// - Otherwise: numeric part + 0.0 (e.g., "13" → 13.0)
/// - Returns None for invalid or empty strings
pub(crate) fn fallback_internal_level(level: &str) -> Option<f32> {
    let level = level.trim();
    if level.is_empty() || level == "N/A" {
        return None;
    }

    let has_plus = level.ends_with('+');
    let numeric_part = if has_plus {
        level.trim_end_matches('+')
    } else {
        level
    };

    let base: f32 = numeric_part.trim().parse().ok()?;
    let offset = if has_plus { 0.6 } else { 0.0 };
    Some(base + offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_internal_level_basic() {
        assert_eq!(fallback_internal_level("13"), Some(13.0));
        assert_eq!(fallback_internal_level("14"), Some(14.0));
        assert_eq!(fallback_internal_level("15"), Some(15.0));
    }

    #[test]
    fn test_fallback_internal_level_plus() {
        assert_eq!(fallback_internal_level("13+"), Some(13.6));
        assert_eq!(fallback_internal_level("14+"), Some(14.6));
        assert_eq!(fallback_internal_level("15+"), Some(15.6));
    }

    #[test]
    fn test_fallback_internal_level_edge_cases() {
        assert_eq!(fallback_internal_level(""), None);
        assert_eq!(fallback_internal_level("N/A"), None);
        assert_eq!(fallback_internal_level("  13  "), Some(13.0));
        assert_eq!(fallback_internal_level("  13+  "), Some(13.6));
        assert_eq!(fallback_internal_level("invalid"), None);
    }
}
