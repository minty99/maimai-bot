use crate::BotData;
use crate::chart_links::linked_chart_label;
use crate::client::{RecordCollectorClient, SongCatalogSong};
use crate::embeds::{embed_base, format_level_with_internal};
use crate::emoji::{format_fc, format_rank, format_sync};
use eyre::WrapErr;
use models::{ChartType, DifficultyCategory, ScoreApiResponse};
use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use serenity::builder::{CreateMessage, CreateThread, EditThread};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

type Error = Box<dyn std::error::Error + Send + Sync>;
type PoiseContext<'a> = poise::Context<'a, BotData, Error>;

pub(crate) type UpdownSessionStore = Arc<Mutex<HashMap<serenity::UserId, UpdownSession>>>;

const MIN_LEVEL_TENTHS: i16 = 10;
const MAX_LEVEL_TENTHS: i16 = 150;
const REACTION_DOWN: &str = "⬇️";
const REACTION_STAY: &str = "⏺️";
const REACTION_UP: &str = "⬆️";

#[derive(Debug, Clone)]
pub(crate) struct UpdownSession {
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
    in_flight_pick_message_id: Option<serenity::MessageId>,
    thread_channel_id: serenity::ChannelId,
    current_level_tenths: i16,
    candidate_pools: Arc<HashMap<i16, Vec<UpdownCandidate>>>,
}

#[derive(Debug, Clone)]
struct UpdownCandidate {
    title: String,
    image_name: Option<String>,
    chart_type: ChartType,
    diff_category: DifficultyCategory,
    level: String,
    internal_level: f32,
    score: Option<ScoreApiResponse>,
}

pub(crate) fn new_session_store() -> UpdownSessionStore {
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

    let Some((_pool_size, candidate)) = choose_candidate_at_level(&pools, start_level_tenths)
    else {
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

    let session = UpdownSession {
        user_id: ctx.author().id,
        pick_message_id: pick_message.id,
        in_flight_pick_message_id: None,
        thread_channel_id: thread.id,
        current_level_tenths: start_level_tenths,
        candidate_pools: Arc::new(pools),
    };

    let previous_session =
        lock_session_store(&ctx.data().updown_sessions).insert(ctx.author().id, session);
    if let Some(previous_session) = previous_session {
        archive_session_thread(ctx.serenity_context(), previous_session.thread_channel_id).await;
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
                remove_session_by_thread_id(&data.updown_sessions, new.id);
            }
        }
        serenity::FullEvent::ThreadDelete { thread, .. } => {
            remove_session_by_thread_id(&data.updown_sessions, thread.id);
        }
        _ => {}
    }

    Ok(())
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

    let Some(session) =
        claim_session_by_pick_message(&data.updown_sessions, user_id, reaction.message_id)
    else {
        return Ok(());
    };

    let pools = &session.candidate_pools;

    if delta == 0 {
        if let Some((_pool_size, candidate)) =
            choose_candidate_at_level(pools, session.current_level_tenths)
        {
            let pick_message =
                match send_pick_message(ctx, data, session.thread_channel_id, &candidate, None)
                    .await
                {
                    Ok(message) => message,
                    Err(err) => {
                        let _ = release_session_claim(
                            &data.updown_sessions,
                            session.user_id,
                            session.pick_message_id,
                        );
                        return Err(err);
                    }
                };
            if finish_session_progress(
                &data.updown_sessions,
                session.user_id,
                session.pick_message_id,
                session.current_level_tenths,
                pick_message.id,
            )
            .is_none()
            {
                let _ = release_session_claim(
                    &data.updown_sessions,
                    session.user_id,
                    session.pick_message_id,
                );
                tracing::warn!(
                    "mai-updown session progress update was skipped after reroll for user {} in thread {}",
                    session.user_id,
                    session.thread_channel_id
                );
            }
        } else {
            let _ = release_session_claim(
                &data.updown_sessions,
                session.user_id,
                session.pick_message_id,
            );
            announce_session_notice(
                ctx,
                session.thread_channel_id,
                &format!(
                    "No eligible charts found at **{}** with the current filters. Keeping the current level.",
                    format_level_tenths(session.current_level_tenths)
                ),
            )
            .await?;
        }

        return Ok(());
    }

    let requested_level = session.current_level_tenths + delta;
    match choose_candidate_in_direction(pools, session.current_level_tenths, delta) {
        Some((found_level_tenths, _pool_size, candidate)) => {
            let note = if found_level_tenths == requested_level {
                None
            } else {
                Some(format!(
                    "No eligible chart at **{}**. Jumped to **{}** instead.",
                    format_level_tenths(requested_level),
                    format_level_tenths(found_level_tenths)
                ))
            };

            let pick_message =
                match send_pick_message(ctx, data, session.thread_channel_id, &candidate, note)
                    .await
                {
                    Ok(message) => message,
                    Err(err) => {
                        let _ = release_session_claim(
                            &data.updown_sessions,
                            session.user_id,
                            session.pick_message_id,
                        );
                        return Err(err);
                    }
                };
            if finish_session_progress(
                &data.updown_sessions,
                session.user_id,
                session.pick_message_id,
                found_level_tenths,
                pick_message.id,
            )
            .is_none()
            {
                let _ = release_session_claim(
                    &data.updown_sessions,
                    session.user_id,
                    session.pick_message_id,
                );
                tracing::warn!(
                    "mai-updown session progress update was skipped after level move for user {} in thread {}",
                    session.user_id,
                    session.thread_channel_id
                );
            }
        }
        None => {
            let _ = release_session_claim(
                &data.updown_sessions,
                session.user_id,
                session.pick_message_id,
            );
            announce_session_notice(
                ctx,
                session.thread_channel_id,
                &format!(
                    "No eligible chart found before leaving the 1.0-15.0 range. Keeping **{}**.",
                    format_level_tenths(session.current_level_tenths)
                ),
            )
            .await?;
        }
    }
    Ok(())
}

async fn build_candidate_pools(
    song_database_client: &crate::client::SongDatabaseClient,
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
) -> Option<(usize, UpdownCandidate)> {
    let candidates = pools.get(&level_tenths)?;
    let mut rng = rand::thread_rng();
    let selected = candidates.choose(&mut rng)?.clone();
    Some((candidates.len(), selected))
}

fn choose_candidate_in_direction(
    pools: &HashMap<i16, Vec<UpdownCandidate>>,
    current_level_tenths: i16,
    delta: i16,
) -> Option<(i16, usize, UpdownCandidate)> {
    let mut next_level = current_level_tenths + delta;
    while (MIN_LEVEL_TENTHS..=MAX_LEVEL_TENTHS).contains(&next_level) {
        if let Some((pool_size, candidate)) = choose_candidate_at_level(pools, next_level) {
            return Some((next_level, pool_size, candidate));
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
    let achievement = candidate
        .score
        .as_ref()
        .and_then(|score| score.achievement_x10000)
        .map(format_rate_x10000)
        .unwrap_or_else(|| "Unplayed".to_string());
    let rank = format_rank(
        &data.status_emojis,
        candidate.score.as_ref().and_then(|score| score.rank),
        "-",
    );
    let fc = format_fc(
        &data.status_emojis,
        candidate.score.as_ref().and_then(|score| score.fc),
        "-",
    );
    let sync = format_sync(
        &data.status_emojis,
        candidate.score.as_ref().and_then(|score| score.sync),
        "-",
    );
    let meta = [
        candidate
            .score
            .as_ref()
            .and_then(|score| score.last_played_at.as_deref())
            .map(|value| format!("Last: {value}")),
        candidate
            .score
            .as_ref()
            .and_then(|score| score.play_count)
            .map(|value| format!("Plays: {value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" • ");

    let mut embed = embed_base(&candidate.title).description(format!(
        "**{chart_line}**\n\
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

fn claim_session_by_pick_message(
    session_store: &UpdownSessionStore,
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
) -> Option<UpdownSession> {
    let mut sessions = lock_session_store(session_store);
    let session = sessions.get_mut(&user_id)?;
    if session.pick_message_id != pick_message_id || session.in_flight_pick_message_id.is_some() {
        return None;
    }

    session.in_flight_pick_message_id = Some(pick_message_id);
    Some(session.clone())
}

fn finish_session_progress(
    session_store: &UpdownSessionStore,
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
    new_level_tenths: i16,
    new_pick_message_id: serenity::MessageId,
) -> Option<UpdownSession> {
    let mut sessions = lock_session_store(session_store);
    let session = sessions.get_mut(&user_id)?;
    if session.pick_message_id != pick_message_id
        || session.in_flight_pick_message_id != Some(pick_message_id)
    {
        return None;
    }

    session.current_level_tenths = new_level_tenths;
    session.pick_message_id = new_pick_message_id;
    session.in_flight_pick_message_id = None;
    Some(session.clone())
}

fn release_session_claim(
    session_store: &UpdownSessionStore,
    user_id: serenity::UserId,
    pick_message_id: serenity::MessageId,
) -> Option<UpdownSession> {
    let mut sessions = lock_session_store(session_store);
    let session = sessions.get_mut(&user_id)?;
    if session.pick_message_id != pick_message_id
        || session.in_flight_pick_message_id != Some(pick_message_id)
    {
        return None;
    }

    session.in_flight_pick_message_id = None;
    Some(session.clone())
}

fn remove_session_by_thread_id(
    session_store: &UpdownSessionStore,
    thread_channel_id: serenity::ChannelId,
) -> Option<UpdownSession> {
    let mut sessions = lock_session_store(session_store);
    let user_id = sessions.iter().find_map(|(user_id, session)| {
        (session.thread_channel_id == thread_channel_id).then_some(*user_id)
    })?;

    sessions.remove(&user_id)
}

fn lock_session_store(
    session_store: &UpdownSessionStore,
) -> MutexGuard<'_, HashMap<serenity::UserId, UpdownSession>> {
    match session_store.lock() {
        Ok(sessions) => sessions,
        Err(poisoned) => {
            tracing::error!("mai-updown session store was poisoned; recovering");
            poisoned.into_inner()
        }
    }
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
