use crate::BotData;
use crate::chart_links::linked_chart_label;
use crate::db;
use crate::embeds::{embed_base, format_level_with_internal};
use crate::emoji::{format_fc, format_rank, format_sync};
use eyre::WrapErr;
use maimai_client::{RecordCollectorClient, SongCatalogSong};
use models::{ChartType, DifficultyCategory, ScoreApiResponse};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use serenity::builder::{CreateMessage, CreateThread, EditThread};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use time::OffsetDateTime;

type Error = Box<dyn std::error::Error + Send + Sync>;
type PoiseContext<'a> = poise::Context<'a, BotData, Error>;

/// In-memory lock table preventing concurrent reaction handling for the same
/// user's session. Maps a user ID to the pick message ID currently being
/// processed. Session metadata itself lives in SQLite (`updown_sessions`).
pub(crate) type UpdownInFlightLocks = Arc<Mutex<HashMap<serenity::UserId, serenity::MessageId>>>;

const MIN_LEVEL_TENTHS: i16 = 10;
const MAX_LEVEL_TENTHS: i16 = 150;
const REACTION_DOWN: &str = "⬇️";
const REACTION_STAY: &str = "⏺️";
const REACTION_UP: &str = "⬆️";

#[derive(Debug, Clone)]
struct UpdownCandidate {
    title: String,
    image_name: Option<String>,
    version: Option<String>,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
    level: String,
    internal_level: f32,
    score: Option<ScoreApiResponse>,
}

pub(crate) fn new_in_flight_locks() -> UpdownInFlightLocks {
    Arc::new(Mutex::new(HashMap::new()))
}

pub(crate) fn parse_level_tenths(value: f64) -> eyre::Result<i16> {
    eyre::ensure!(value.is_finite(), "Internal level must be a number.");

    let scaled = value * 10.0;
    let rounded = scaled.round();
    eyre::ensure!(
        (scaled - rounded).abs() < 1e-6,
        "Internal level must use 0.1 increments, for example `13.0`."
    );

    let tenths = rounded as i16;
    eyre::ensure!(
        (MIN_LEVEL_TENTHS..=MAX_LEVEL_TENTHS).contains(&tenths),
        "Internal level must be between 1.0 and 15.0."
    );

    Ok(tenths)
}

pub(crate) async fn start_session(
    ctx: PoiseContext<'_>,
    record_collector_client: RecordCollectorClient,
    start_level_tenths: i16,
) -> Result<(), Error> {
    ensure_start_channel_supported(ctx).await?;

    let pools =
        build_candidate_pools(&ctx.data().song_database_client, &record_collector_client).await?;

    let Some(candidate) = choose_candidate_at_level(&pools, start_level_tenths) else {
        return Err(eyre::eyre!(
            "No eligible charts found at internal level **{}** with the current filters.",
            format_level_tenths(start_level_tenths)
        )
        .into());
    };

    let root_message = ctx
        .channel_id()
        .send_message(
            ctx.serenity_context(),
            CreateMessage::new().embed(build_session_intro_embed(
                ctx.author().id,
                start_level_tenths,
            )),
        )
        .await
        .inspect_err(|err| tracing::error!("{err:?}"))
        .wrap_err("send mai-updown root message")?;

    let thread_name = format!("mai-updown {}", format_level_tenths(start_level_tenths));
    let thread = ctx
        .channel_id()
        .create_thread_from_message(
            ctx.serenity_context(),
            root_message.id,
            CreateThread::new(thread_name)
                .auto_archive_duration(serenity::AutoArchiveDuration::OneHour),
        )
        .await
        .inspect_err(|err| tracing::error!("{err:?}"))
        .wrap_err("create mai-updown thread")?;

    let pick_message = send_pick_message(
        ctx.serenity_context(),
        ctx.data(),
        thread.id,
        &candidate,
        None,
    )
    .await?;

    let previous = db::get_updown_session(&ctx.data().db_pool, ctx.author().id)
        .await
        .wrap_err("load previous mai-updown session")?;

    {
        let mut guard = lock_in_flight(&ctx.data().updown_in_flight);
        guard.remove(&ctx.author().id);
    }

    db::upsert_updown_session(
        &ctx.data().db_pool,
        ctx.author().id,
        thread.id,
        pick_message.id,
        start_level_tenths,
        OffsetDateTime::now_utc().unix_timestamp(),
    )
    .await
    .wrap_err("persist mai-updown session")?;

    if let Some(prev) = previous
        && prev.thread_channel_id != thread.id
    {
        archive_session_thread(ctx.serenity_context(), prev.thread_channel_id).await;
    }

    Ok(())
}

pub(crate) async fn handle_event(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &BotData,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::ReactionAdd { add_reaction } => {
            handle_reaction_add(ctx, data, add_reaction).await?;
        }
        serenity::FullEvent::ThreadUpdate { new, .. } => {
            if new
                .thread_metadata
                .as_ref()
                .is_some_and(|metadata| metadata.archived)
            {
                cleanup_session_for_thread(data, new.id).await;
            }
        }
        serenity::FullEvent::ThreadDelete { thread, .. } => {
            cleanup_session_for_thread(data, thread.id).await;
        }
        _ => {}
    }

    Ok(())
}

async fn cleanup_session_for_thread(data: &BotData, thread_channel_id: serenity::ChannelId) {
    if let Err(err) = db::delete_updown_session_by_thread(&data.db_pool, thread_channel_id).await {
        tracing::warn!("delete mai-updown session row failed: {err:?}");
    }
}

async fn handle_reaction_add(
    ctx: &serenity::Context,
    data: &BotData,
    reaction: &serenity::Reaction,
) -> Result<(), Error> {
    let Some(user_id) = reaction.user_id else {
        return Ok(());
    };
    if user_id == ctx.cache.current_user().id {
        return Ok(());
    }
    if reaction
        .member
        .as_ref()
        .is_some_and(|member| member.user.bot)
        || ctx.cache.user(user_id).is_some_and(|user| user.bot)
    {
        return Ok(());
    }

    let Some(delta) = reaction_delta(&reaction.emoji) else {
        return Ok(());
    };

    let Some(session) = db::get_updown_session(&data.db_pool, user_id)
        .await
        .wrap_err("load mai-updown session")?
    else {
        return Ok(());
    };
    if session.pick_message_id != reaction.message_id {
        return Ok(());
    }

    if !try_acquire_in_flight(&data.updown_in_flight, user_id, session.pick_message_id) {
        return Ok(());
    }

    let result = process_reaction(ctx, data, &session, delta).await;
    release_in_flight(&data.updown_in_flight, user_id, session.pick_message_id);
    result
}

async fn process_reaction(
    ctx: &serenity::Context,
    data: &BotData,
    session: &db::PersistedUpdownSession,
    delta: i16,
) -> Result<(), Error> {
    let registration = db::get_registration(&data.db_pool, session.discord_user_id)
        .await
        .wrap_err("load user registration")?
        .ok_or_else(|| eyre::eyre!("no record collector registered"))?;
    let record_collector_client =
        RecordCollectorClient::new(registration.record_collector_server_url)
            .wrap_err("build record collector client")?;

    let pools = build_candidate_pools(&data.song_database_client, &record_collector_client).await?;

    let (new_level_tenths, candidate, note) =
        match pick_next_candidate(&pools, session.current_level_tenths, delta) {
            Ok(result) => result,
            Err(notice_msg) => {
                announce_session_notice(ctx, session.thread_channel_id, &notice_msg).await?;
                return Ok(());
            }
        };

    let pick_message =
        send_pick_message(ctx, data, session.thread_channel_id, &candidate, note).await?;

    let affected = db::update_updown_session_progress(
        &data.db_pool,
        session.discord_user_id,
        session.thread_channel_id,
        pick_message.id,
        new_level_tenths,
        OffsetDateTime::now_utc().unix_timestamp(),
    )
    .await
    .wrap_err("persist mai-updown session progress")?;

    if affected == 0 {
        tracing::info!(
            "mai-updown session row for user {} was removed during reaction processing; not resurrecting",
            session.discord_user_id
        );
    }

    Ok(())
}

async fn build_candidate_pools(
    song_database_client: &maimai_client::SongDatabaseClient,
    record_collector_client: &RecordCollectorClient,
) -> eyre::Result<HashMap<i16, Vec<UpdownCandidate>>> {
    let scores = record_collector_client
        .get_all_rated_scores()
        .await
        .wrap_err("fetch rated scores")?;
    let mut score_map = HashMap::with_capacity(scores.len());
    for score in scores {
        score_map.insert(
            chart_identity_key(
                &score.title,
                &score.genre,
                &score.artist,
                score.chart_type,
                score.diff_category,
            ),
            score,
        );
    }

    let songs = song_database_client
        .list_song_catalog()
        .await
        .wrap_err("load song catalog")?;

    let mut pools: HashMap<i16, Vec<UpdownCandidate>> = HashMap::new();
    for song in songs {
        append_song_candidates(&mut pools, &song, &score_map);
    }

    Ok(pools)
}

async fn ensure_start_channel_supported(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let channel = ctx
        .channel_id()
        .to_channel(ctx.serenity_context())
        .await
        .wrap_err("load mai-updown channel")?;

    let Some(channel) = channel.guild() else {
        return Ok(());
    };

    if channel.thread_metadata.is_some() {
        return Err(eyre::eyre!(
            "mai-updown can only be started from a regular server channel, not inside an existing thread."
        )
        .into());
    }

    Ok(())
}

fn append_song_candidates(
    pools: &mut HashMap<i16, Vec<UpdownCandidate>>,
    song: &SongCatalogSong,
    score_map: &HashMap<String, ScoreApiResponse>,
) {
    for sheet in &song.sheets {
        if !sheet.region.intl {
            continue;
        }

        let Some(internal_level) = sheet.internal_level else {
            continue;
        };
        let level_tenths = internal_level_tenths(internal_level);

        let score_key = chart_identity_key(
            &song.title,
            &song.genre,
            &song.artist,
            sheet.chart_type,
            sheet.diff_category,
        );
        let score = score_map.get(&score_key).cloned();

        pools
            .entry(level_tenths)
            .or_default()
            .push(UpdownCandidate {
                title: song.title.clone(),
                image_name: song.image_name.clone(),
                version: sheet.version.clone(),
                chart_type: sheet.chart_type,
                diff_category: sheet.diff_category,
                level: sheet.level.clone(),
                internal_level,
                score,
            });
    }
}

fn choose_candidate_at_level(
    pools: &HashMap<i16, Vec<UpdownCandidate>>,
    level_tenths: i16,
) -> Option<UpdownCandidate> {
    let candidates = pools.get(&level_tenths)?;
    let mut rng = rand::thread_rng();
    candidates.choose(&mut rng).cloned()
}

fn pick_next_candidate(
    pools: &HashMap<i16, Vec<UpdownCandidate>>,
    current_level_tenths: i16,
    delta: i16,
) -> Result<(i16, UpdownCandidate, Option<String>), String> {
    if delta == 0 {
        return match choose_candidate_at_level(pools, current_level_tenths) {
            Some(candidate) => Ok((current_level_tenths, candidate, None)),
            None => Err(format!(
                "No eligible charts found at **{}** with the current filters. Keeping the current level.",
                format_level_tenths(current_level_tenths)
            )),
        };
    }

    let requested_level = current_level_tenths + delta;
    match choose_candidate_in_direction(pools, current_level_tenths, delta) {
        Some((found_level_tenths, candidate)) => {
            let note = (found_level_tenths != requested_level).then(|| {
                format!(
                    "No eligible chart at **{}**. Jumped to **{}** instead.",
                    format_level_tenths(requested_level),
                    format_level_tenths(found_level_tenths)
                )
            });
            Ok((found_level_tenths, candidate, note))
        }
        None => Err(format!(
            "No eligible chart found before leaving the 1.0-15.0 range. Keeping **{}**.",
            format_level_tenths(current_level_tenths)
        )),
    }
}

fn choose_candidate_in_direction(
    pools: &HashMap<i16, Vec<UpdownCandidate>>,
    current_level_tenths: i16,
    delta: i16,
) -> Option<(i16, UpdownCandidate)> {
    let mut next_level = current_level_tenths + delta;
    while (MIN_LEVEL_TENTHS..=MAX_LEVEL_TENTHS).contains(&next_level) {
        if let Some(candidate) = choose_candidate_at_level(pools, next_level) {
            return Some((next_level, candidate));
        }
        next_level += delta;
    }

    None
}

fn build_session_intro_embed(
    user_id: serenity::UserId,
    start_level_tenths: i16,
) -> serenity::CreateEmbed {
    embed_base("mai-updown started").description(format!(
        "Started by <@{}>\n\
         Start level: **{}**\n\
         Controls: {REACTION_DOWN} `-0.1` • {REACTION_STAY} `±0.0` • {REACTION_UP} `+0.1`",
        user_id.get(),
        format_level_tenths(start_level_tenths),
    ))
}

fn build_pick_embed(data: &BotData, candidate: &UpdownCandidate) -> serenity::CreateEmbed {
    let level = format_level_with_internal(&candidate.level, Some(candidate.internal_level));
    let chart_line = linked_chart_label(
        &candidate.title,
        candidate.chart_type,
        candidate.diff_category,
        &level,
    );
    let version_line = candidate
        .version
        .as_deref()
        .map(|version| format!("Version: {version}"))
        .unwrap_or_else(|| "Version: -".to_string());
    let score = candidate.score.as_ref();
    let achievement = score
        .and_then(|s| s.achievement_x10000)
        .map(format_rate_x10000)
        .unwrap_or_else(|| "Unplayed".to_string());
    let rank = format_rank(&data.status_emojis, score.and_then(|s| s.rank), "-");
    let fc = format_fc(&data.status_emojis, score.and_then(|s| s.fc), "-");
    let sync = format_sync(&data.status_emojis, score.and_then(|s| s.sync), "-");
    let meta = [
        score
            .and_then(|s| s.last_played_at.as_deref())
            .map(|value| format!("Last: {value}")),
        score
            .and_then(|s| s.play_count)
            .map(|value| format!("Plays: {value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" • ");

    let mut embed = embed_base(&candidate.title).description(format!(
        "**{chart_line}**\n\
         {version_line}\n\
         {achievement} • {rank} • {fc} • {sync}\n\
         {meta}"
    ));
    if let Some(image_name) = candidate.image_name.as_deref() {
        embed = embed.thumbnail(data.song_database_client.cover_url(image_name));
    }
    embed
}

async fn send_pick_message(
    cache_http: impl serenity::CacheHttp,
    data: &BotData,
    thread_channel_id: serenity::ChannelId,
    candidate: &UpdownCandidate,
    note: Option<String>,
) -> Result<serenity::Message, Error> {
    let mut builder = CreateMessage::new().embed(build_pick_embed(data, candidate));
    if let Some(note) = note {
        builder = builder.content(note);
    }

    let message = thread_channel_id
        .send_message(&cache_http, builder)
        .await
        .wrap_err("send mai-updown pick message")?;

    for emoji in [REACTION_DOWN, REACTION_STAY, REACTION_UP] {
        if let Err(err) = message
            .react(
                cache_http.http(),
                serenity::ReactionType::Unicode(emoji.to_string()),
            )
            .await
        {
            tracing::error!("{err:?}");
            if let Err(delete_err) = message.delete(cache_http.http()).await {
                tracing::warn!(
                    "failed to delete incomplete mai-updown pick message: {delete_err:#}"
                );
            }
            return Err(eyre::eyre!("add mai-updown pick reaction: {err}").into());
        }
    }

    Ok(message)
}

async fn announce_session_notice(
    cache_http: impl serenity::CacheHttp,
    thread_channel_id: serenity::ChannelId,
    message: &str,
) -> Result<(), Error> {
    thread_channel_id
        .say(cache_http, message)
        .await
        .wrap_err("send mai-updown session notice")?;

    Ok(())
}

async fn archive_session_thread(
    cache_http: impl serenity::CacheHttp,
    thread_channel_id: serenity::ChannelId,
) {
    if let Err(err) = thread_channel_id
        .edit_thread(cache_http, EditThread::new().archived(true))
        .await
    {
        tracing::warn!(
            "failed to archive previous mai-updown thread {}: {err:#}",
            thread_channel_id
        );
    }
}

fn try_acquire_in_flight(
    locks: &UpdownInFlightLocks,
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
) -> bool {
    let mut guard = lock_in_flight(locks);
    match guard.entry(user_id) {
        std::collections::hash_map::Entry::Occupied(_) => false,
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(pick_message_id);
            true
        }
    }
}

fn release_in_flight(
    locks: &UpdownInFlightLocks,
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
) {
    let mut guard = lock_in_flight(locks);
    if guard.get(&user_id) == Some(&pick_message_id) {
        guard.remove(&user_id);
    }
}

fn lock_in_flight(
    locks: &UpdownInFlightLocks,
) -> MutexGuard<'_, HashMap<serenity::UserId, serenity::MessageId>> {
    locks.lock().expect("mai-updown in-flight lock")
}

fn internal_level_tenths(value: f32) -> i16 {
    (value as f64 * 10.0).round() as i16
}

fn chart_identity_key(
    title: &str,
    genre: &str,
    artist: &str,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
) -> String {
    format!(
        "{title}\u{1f}{genre}\u{1f}{artist}\u{1f}{}\u{1f}{}",
        chart_type.as_str(),
        diff_category.as_str(),
    )
}

fn format_level_tenths(level_tenths: i16) -> String {
    format!("{:.1}", level_tenths as f64 / 10.0)
}

fn format_rate_x10000(value: i64) -> String {
    format!("{:.4}%", value as f64 / 10000.0)
}

fn reaction_delta(emoji: &serenity::ReactionType) -> Option<i16> {
    if emoji.unicode_eq(REACTION_DOWN) {
        Some(-1)
    } else if emoji.unicode_eq(REACTION_STAY) {
        Some(0)
    } else if emoji.unicode_eq(REACTION_UP) {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{MIN_LEVEL_TENTHS, parse_level_tenths, reaction_delta};
    use poise::serenity_prelude as serenity;

    #[test]
    fn parse_level_requires_one_decimal_step() {
        assert_eq!(parse_level_tenths(13.0).unwrap(), 130);
        assert_eq!(parse_level_tenths(1.0).unwrap(), MIN_LEVEL_TENTHS);
        assert!(parse_level_tenths(13.05).is_err());
        assert!(parse_level_tenths(15.1).is_err());
    }

    #[test]
    fn reaction_delta_matches_controls() {
        assert_eq!(
            reaction_delta(&serenity::ReactionType::Unicode("⬇️".to_string())),
            Some(-1)
        );
        assert_eq!(
            reaction_delta(&serenity::ReactionType::Unicode("⏺️".to_string())),
            Some(0)
        );
        assert_eq!(
            reaction_delta(&serenity::ReactionType::Unicode("⬆️".to_string())),
            Some(1)
        );
    }
}
