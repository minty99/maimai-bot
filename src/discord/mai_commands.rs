use eyre::{Result, WrapErr};
use ordered_float::OrderedFloat;
use poise::serenity_prelude as serenity;
use serenity::builder::CreateEmbed;

use crate::db::SqlitePool;
use crate::discord::bot::{
    RecentOptionalFields, RecentRecordView, ScoreRowView, build_mai_recent_embeds,
    build_mai_score_embed, build_mai_today_embed, embed_base, format_level_with_internal,
    latest_credit_len,
};
use crate::maimai::rating::{chart_rating_points, is_ap_like};
use crate::song_data::{SongBucket, SongDataIndex};

#[derive(Debug, Clone)]
pub(crate) struct TodayDetailRowView {
    pub(crate) title: String,
    pub(crate) chart_type: String,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) achievement_new_record: bool,
    pub(crate) first_play: bool,
}

pub(crate) fn build_mai_today_detail_embed(
    display_name: &str,
    day_key: &str,
    start: &str,
    end: &str,
    rows: &[TodayDetailRowView],
) -> CreateEmbed {
    let mut desc = String::new();
    let total = rows.len();

    for (idx, row) in rows.iter().enumerate() {
        let achv = crate::db::format_percent_f64(row.achievement_percent);
        let mut line = format!("- **{}** [{}] — {}", row.title, row.chart_type, achv);
        if let Some(pt) = row.rating_points {
            line.push_str(&format!(" • {pt}pt"));
        }
        if row.first_play {
            line.push_str(" [FIRST PLAY]");
        } else if row.achievement_new_record {
            line.push_str(" [NEW RECORD]");
        }
        line.push('\n');

        // Discord embed description max is 4096 chars; keep some room for a truncation line.
        if desc.len().saturating_add(line.len()) > 3900 {
            desc.push_str(&format!("... (truncated; showing {}/{total})\n", idx));
            break;
        }
        desc.push_str(&line);
    }

    if desc.trim().is_empty() {
        desc = "No playlogs found for this day.".to_string();
    }

    embed_base(&format!("{display_name}'s plays on {day_key}"))
        .field("Window", format!("{start} ~ {end}"), false)
        .description(desc)
}

pub(crate) async fn build_mai_today_detail_embed_for_day(
    pool: &SqlitePool,
    song_data: Option<&SongDataIndex>,
    display_name: &str,
    day_key: &str,
    start: &str,
    end: &str,
) -> Result<CreateEmbed> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<f64>,
            Option<String>,
            Option<String>,
            i64,
            i64,
        ),
    >(
        r#"
        SELECT
            pl.title,
            pl.chart_type,
            pl.achievement_x10000 / 10000.0 as achievement_percent,
            pl.diff_category,
            pl.fc,
            pl.achievement_new_record,
            pl.first_play
        FROM playlogs pl
        WHERE pl.played_at >= ?1
          AND pl.played_at < ?2
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .wrap_err("query playlogs for day")?;

    let mut out = rows
        .into_iter()
        .map(
            |(
                title,
                chart_type,
                achievement,
                diff_category,
                fc,
                achievement_new_record,
                first_play,
            )| {
                let internal_level = diff_category.as_deref().and_then(|diff| {
                    song_data.and_then(|idx| idx.internal_level(&title, &chart_type, diff))
                });
                let rating_points = internal_level.and_then(|internal| {
                    let ach = achievement?;
                    let ap = is_ap_like(fc.as_deref());
                    Some(chart_rating_points(internal as f64, ach, ap))
                });
                TodayDetailRowView {
                    title,
                    chart_type,
                    achievement_percent: achievement,
                    rating_points,
                    achievement_new_record: achievement_new_record != 0,
                    first_play: first_play != 0,
                }
            },
        )
        .collect::<Vec<_>>();

    out.sort_by_key(|r| std::cmp::Reverse(r.rating_points.unwrap_or(0)));

    Ok(build_mai_today_detail_embed(
        display_name,
        day_key,
        start,
        end,
        &out,
    ))
}

pub(crate) async fn fetch_score_titles(pool: &SqlitePool) -> Result<Vec<String>> {
    sqlx::query_scalar::<_, String>("SELECT DISTINCT title FROM scores")
        .fetch_all(pool)
        .await
        .wrap_err("query titles")
}

pub(crate) fn embed_no_scores_found() -> CreateEmbed {
    embed_base("No scores found")
        .description("DB has no `scores` yet. Run the bot once to build it first.")
}

pub(crate) async fn build_mai_score_embed_for_title(
    pool: &SqlitePool,
    song_data: Option<&SongDataIndex>,
    display_name: &str,
    title: &str,
) -> Result<(CreateEmbed, bool)> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            Option<f64>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            sc.chart_type,
            sc.diff_category,
            sc.level,
            sc.achievement_x10000 / 10000.0 as achievement_percent,
            sc.rank,
            sc.fc
        FROM scores sc
        WHERE sc.title = ?
          AND sc.achievement_x10000 IS NOT NULL
        ORDER BY
            CASE sc.chart_type
                WHEN 'STD' THEN 0
                WHEN 'DX' THEN 1
                ELSE 255
            END,
            CASE sc.diff_category
                WHEN 'BASIC' THEN 0
                WHEN 'ADVANCED' THEN 1
                WHEN 'EXPERT' THEN 2
                WHEN 'MASTER' THEN 3
                WHEN 'Re:MASTER' THEN 4
                ELSE 255
            END,
            sc.level
        "#,
    )
    .bind(title)
    .fetch_all(pool)
    .await
    .wrap_err("query scores")?;

    if rows.is_empty() {
        return Ok((
            embed_base("No records found").description("No score rows found."),
            false,
        ));
    }

    let entries = rows
        .into_iter()
        .map(
            |(chart_type, diff_category, level, achievement, rank, fc)| {
                let internal_level = song_data
                    .and_then(|idx| idx.internal_level(title, &chart_type, &diff_category));
                let rating_points = internal_level.and_then(|internal| {
                    let ach = achievement?;
                    let ap = is_ap_like(fc.as_deref());
                    Some(chart_rating_points(internal as f64, ach, ap))
                });
                ScoreRowView {
                    chart_type,
                    diff_category,
                    level,
                    internal_level,
                    rating_points,
                    achievement_percent: achievement,
                    rank,
                }
            },
        )
        .collect::<Vec<_>>();

    Ok((build_mai_score_embed(display_name, title, &entries), true))
}

pub(crate) async fn build_mai_recent_embeds_for_latest_credit(
    pool: &SqlitePool,
    song_data: Option<&SongDataIndex>,
    display_name: &str,
    optional_fields: Option<&RecentOptionalFields>,
) -> Result<Vec<CreateEmbed>> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f64>,
            i64,
            i64,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            pl.title,
            pl.chart_type,
            pl.track,
            pl.played_at,
            pl.diff_category,
            pl.level,
            pl.achievement_x10000 / 10000.0 as achievement_percent,
            pl.achievement_new_record,
            pl.first_play,
            pl.score_rank,
            pl.fc
        FROM playlogs pl
        WHERE pl.played_at_unixtime IS NOT NULL
        ORDER BY pl.played_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(pool)
    .await
    .wrap_err("query playlogs")?;

    if rows.is_empty() {
        return Ok(vec![embed_base("No recent records found")]);
    }

    let take = latest_credit_len(&rows.iter().map(|row| row.2).collect::<Vec<_>>());
    let mut recent = rows.into_iter().take(take).collect::<Vec<_>>();
    recent.reverse();

    let records = recent
        .into_iter()
        .map(
            |(
                title,
                chart_type,
                track,
                played_at,
                diff_category,
                level,
                achievement,
                achievement_new_record,
                first_play,
                rank,
                fc,
            )| {
                let internal_level = diff_category.as_deref().and_then(|diff| {
                    song_data.and_then(|idx| idx.internal_level(&title, &chart_type, diff))
                });
                let rating_points = internal_level.and_then(|internal| {
                    let ach = achievement?;
                    let ap = is_ap_like(fc.as_deref());
                    Some(chart_rating_points(internal as f64, ach, ap))
                });
                RecentRecordView {
                    track,
                    played_at,
                    title,
                    chart_type,
                    diff_category,
                    level,
                    internal_level,
                    rating_points,
                    achievement_percent: achievement,
                    achievement_new_record: achievement_new_record != 0,
                    first_play: first_play != 0,
                    rank,
                }
            },
        )
        .collect::<Vec<_>>();

    Ok(build_mai_recent_embeds(
        display_name,
        &records,
        optional_fields,
        song_data,
    ))
}

pub(crate) async fn build_mai_today_embed_for_now(
    pool: &SqlitePool,
    display_name: &str,
) -> Result<CreateEmbed> {
    use time::{Duration as TimeDuration, OffsetDateTime, UtcOffset};

    let offset = UtcOffset::from_hms(9, 0, 0).unwrap_or(UtcOffset::UTC);
    let now_jst = OffsetDateTime::now_utc().to_offset(offset);

    let day_date = if now_jst.hour() < 4 {
        (now_jst - TimeDuration::days(1)).date()
    } else {
        now_jst.date()
    };
    let end_date = day_date + TimeDuration::days(1);

    let day_key = format!(
        "{:04}/{:02}/{:02}",
        day_date.year(),
        u8::from(day_date.month()),
        day_date.day()
    );
    let end_key = format!(
        "{:04}/{:02}/{:02}",
        end_date.year(),
        u8::from(end_date.month()),
        end_date.day()
    );

    let start = format!("{} 04:00", day_key);
    let end = format!("{} 04:00", end_key);

    let (tracks, credits, first_plays, new_record_flags) =
        sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
        SELECT
            COUNT(*) as tracks,
            COUNT(DISTINCT credit_play_count) as credits,
            COALESCE(SUM(first_play), 0) as first_plays,
            COALESCE(SUM(achievement_new_record), 0) as new_record_flags
        FROM playlogs
        WHERE played_at >= ?1
          AND played_at < ?2
        "#,
        )
        .bind(&start)
        .bind(&end)
        .fetch_one(pool)
        .await
        .wrap_err("query today summary")?;

    let new_records_true = (new_record_flags - first_plays).max(0);

    Ok(build_mai_today_embed(
        display_name,
        &start,
        &end,
        credits,
        tracks,
        new_records_true,
        first_plays,
    ))
}

pub(crate) async fn build_mai_rating_embeds(
    pool: &SqlitePool,
    song_data: Option<&SongDataIndex>,
    display_name: &str,
) -> Result<Vec<CreateEmbed>> {
    let Some(song_data) = song_data else {
        return Ok(vec![
            embed_base("song data not loaded")
                .description("Cannot compute rating without song metadata."),
        ]);
    };

    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<f64>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT
            sc.title,
            sc.chart_type,
            sc.diff_category,
            sc.level,
            sc.achievement_x10000 / 10000.0 as achievement_percent,
            sc.rank,
            sc.fc
        FROM scores sc
        WHERE sc.achievement_x10000 IS NOT NULL
        "#,
    )
    .fetch_all(pool)
    .await
    .wrap_err("query scores")?;

    #[derive(Debug, Clone)]
    struct RatedRow {
        bucket: SongBucket,
        title: String,
        chart_type: String,
        diff_category: String,
        level: String,
        internal_level: f32,
        achievement_percent: f64,
        rank: Option<String>,
        rating_points: u32,
    }

    let mut missing_internal = 0usize;
    let mut missing_bucket = 0usize;
    let mut out_rows = Vec::new();

    for (title, chart_type, diff_category, level, achievement, rank, fc) in rows {
        let Some(achievement) = achievement else {
            continue;
        };
        let Some(bucket) = song_data.bucket(&title) else {
            missing_bucket += 1;
            continue;
        };
        let Some(internal_level) = song_data.internal_level(&title, &chart_type, &diff_category)
        else {
            missing_internal += 1;
            continue;
        };

        let ap = is_ap_like(fc.as_deref());
        let rating_points = chart_rating_points(internal_level as f64, achievement, ap);

        out_rows.push(RatedRow {
            bucket,
            title,
            chart_type,
            diff_category,
            level,
            internal_level,
            achievement_percent: achievement,
            rank,
            rating_points,
        });
    }

    let mut new_rows = out_rows
        .iter()
        .filter(|r| r.bucket == SongBucket::New)
        .cloned()
        .collect::<Vec<_>>();
    let mut old_rows = out_rows
        .iter()
        .filter(|r| r.bucket == SongBucket::Old)
        .cloned()
        .collect::<Vec<_>>();

    new_rows
        .sort_by_key(|r| std::cmp::Reverse((r.rating_points, OrderedFloat(r.achievement_percent))));
    old_rows
        .sort_by_key(|r| std::cmp::Reverse((r.rating_points, OrderedFloat(r.achievement_percent))));

    let new_rows = new_rows.into_iter().take(15).collect::<Vec<_>>();
    let old_rows = old_rows.into_iter().take(35).collect::<Vec<_>>();

    let new_sum = new_rows.iter().map(|r| r.rating_points).sum::<u32>();
    let old_sum = old_rows.iter().map(|r| r.rating_points).sum::<u32>();
    let total = new_sum.saturating_add(old_sum);

    fn list_desc(rows: &[RatedRow]) -> String {
        let mut out = String::new();
        for (idx, r) in rows.iter().enumerate() {
            let rank = r.rank.as_deref().unwrap_or("N/A");
            let level = format_level_with_internal(&r.level, Some(r.internal_level));
            out.push_str(&format!(
                "- [{}] `{:>3}pt` {} [{}] {} {} — {:.4}% • {}\n",
                idx + 1,
                r.rating_points,
                r.title,
                r.chart_type,
                r.diff_category,
                level,
                r.achievement_percent,
                rank
            ));
        }
        out
    }

    let mut summary = embed_base(&format!("{}'s rating", display_name))
        .field("Computed", total.to_string(), true)
        .field("NEW 15", new_sum.to_string(), true)
        .field("OLD 35", old_sum.to_string(), true);
    if missing_internal > 0 || missing_bucket > 0 {
        summary = summary.field(
            "Notes",
            format!(
                "missing internal level: {missing_internal}\nmissing song version: {missing_bucket}"
            ),
            false,
        );
    }

    let new_embed = embed_base("NEW 15").description(list_desc(&new_rows));
    let old_embed = embed_base("OLD 35").description(list_desc(&old_rows));

    Ok(vec![summary, new_embed, old_embed])
}
