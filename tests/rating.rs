use maimai_bot::maimai::rating::{chart_rating_points, coefficient_for_achievement};

#[test]
fn coefficient_table_key_breakpoints() {
    assert_eq!(coefficient_for_achievement(0.0), 0.0);
    assert_eq!(coefficient_for_achievement(10.0), 1.6);
    assert_eq!(coefficient_for_achievement(75.0), 12.0);
    assert_eq!(coefficient_for_achievement(79.9999), 12.8);
    assert_eq!(coefficient_for_achievement(80.0), 13.6);
    assert_eq!(coefficient_for_achievement(96.9999), 17.6);
    assert_eq!(coefficient_for_achievement(97.0), 20.0);
    assert_eq!(coefficient_for_achievement(99.9999), 21.4);
    assert_eq!(coefficient_for_achievement(100.0), 21.6);
    assert_eq!(coefficient_for_achievement(100.4999), 22.2);
    assert_eq!(coefficient_for_achievement(100.5), 22.4);
    assert_eq!(coefficient_for_achievement(101.0), 22.4);
}

#[test]
fn chart_rating_matches_known_examples() {
    // Example from https://listed.to/@donmai/45107
    let internal = 14.4;

    let r1 = chart_rating_points(internal, 99.8056, false);
    assert_eq!(r1, 303);

    let r2 = chart_rating_points(internal, 99.9999, false);
    assert_eq!(r2, 308);
}

#[test]
fn chart_rating_ap_bonus_adds_one() {
    let internal = 14.4;
    let base = chart_rating_points(internal, 99.8056, false);
    let ap = chart_rating_points(internal, 99.8056, true);
    assert_eq!(ap, base + 1);
}
