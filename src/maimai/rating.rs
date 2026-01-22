use crate::song_data::SongBucket;

pub const ACHIEVEMENT_CAP: f64 = 100.5;

pub fn coefficient_for_achievement(achievement_percent: f64) -> f64 {
    // Coefficient table for Gen 3 / 3.5.
    // Source: https://silentblue.remywiki.com/maimai_DX:Rating (Gen 3),
    // cross-checked with https://github.com/gekichumai/dxrating/blob/0c5cce11/apps/web/src/utils/rating.ts
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

pub fn chart_rating_points(internal_level: f64, achievement_percent: f64, ap_bonus: bool) -> u32 {
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

pub fn is_ap_like(fc: Option<&str>) -> bool {
    matches!(fc, Some("AP") | Some("AP+"))
}

pub fn bucket_label(bucket: SongBucket) -> &'static str {
    match bucket {
        SongBucket::New => "NEW",
        SongBucket::Old => "OLD",
    }
}
