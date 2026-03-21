use std::collections::HashMap;

use base64::Engine as _;
use eyre::{Result, WrapErr};
use models::{FcStatus, ScoreRank, SyncStatus};
use poise::serenity_prelude as serenity;
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub(crate) struct MaimaiStatusEmojis {
    mentions: HashMap<&'static str, String>,
}

impl MaimaiStatusEmojis {
    pub(crate) fn rank(&self, value: ScoreRank) -> Option<&str> {
        self.mentions
            .get(rank_emoji_name(value))
            .map(String::as_str)
    }

    pub(crate) fn fc(&self, value: FcStatus) -> Option<&str> {
        self.mentions.get(fc_emoji_name(value)).map(String::as_str)
    }

    pub(crate) fn sync(&self, value: SyncStatus) -> Option<&str> {
        self.mentions
            .get(sync_emoji_name(value))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, Copy)]
struct EmojiAsset {
    name: &'static str,
    file_name: &'static str,
    bytes: &'static [u8],
}

const STATUS_EMOJI_ASSETS: [EmojiAsset; 23] = [
    EmojiAsset {
        name: "maimai_rank_sssp",
        file_name: "music_icon_sssp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_sssp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_sss",
        file_name: "music_icon_sss.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_sss.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_ssp",
        file_name: "music_icon_ssp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_ssp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_ss",
        file_name: "music_icon_ss.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_ss.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_sp",
        file_name: "music_icon_sp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_sp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_s",
        file_name: "music_icon_s.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_s.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_aaa",
        file_name: "music_icon_aaa.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_aaa.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_aa",
        file_name: "music_icon_aa.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_aa.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_a",
        file_name: "music_icon_a.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_a.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_bbb",
        file_name: "music_icon_bbb.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_bbb.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_bb",
        file_name: "music_icon_bb.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_bb.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_b",
        file_name: "music_icon_b.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_b.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_c",
        file_name: "music_icon_c.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_c.png"
        )),
    },
    EmojiAsset {
        name: "maimai_rank_d",
        file_name: "music_icon_d.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_d.png"
        )),
    },
    EmojiAsset {
        name: "maimai_fc_app",
        file_name: "music_icon_app.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_app.png"
        )),
    },
    EmojiAsset {
        name: "maimai_fc_ap",
        file_name: "music_icon_ap.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_ap.png"
        )),
    },
    EmojiAsset {
        name: "maimai_fc_fcp",
        file_name: "music_icon_fcp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fcp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_fc_fc",
        file_name: "music_icon_fc.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fc.png"
        )),
    },
    EmojiAsset {
        name: "maimai_sync_fdxp",
        file_name: "music_icon_fdxp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fdxp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_sync_fdx",
        file_name: "music_icon_fdx.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fdx.png"
        )),
    },
    EmojiAsset {
        name: "maimai_sync_fsp",
        file_name: "music_icon_fsp.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fsp.png"
        )),
    },
    EmojiAsset {
        name: "maimai_sync_fs",
        file_name: "music_icon_fs.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_fs.png"
        )),
    },
    EmojiAsset {
        name: "maimai_sync_sync",
        file_name: "music_icon_sync.png",
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/status-emojis/music_icon_sync.png"
        )),
    },
];

pub(crate) async fn sync_application_emojis(http: &serenity::Http) -> Result<MaimaiStatusEmojis> {
    let existing = http
        .get_application_emojis()
        .await
        .wrap_err("list application emojis")?;
    let existing_by_name = existing
        .into_iter()
        .map(|emoji| (emoji.name.clone(), emoji))
        .collect::<HashMap<_, _>>();

    let mut mentions = HashMap::new();
    for asset in STATUS_EMOJI_ASSETS {
        if let Some(existing) = existing_by_name.get(asset.name)
            && let Err(error) = http.delete_application_emoji(existing.id).await
        {
            warn!(
                "failed to delete stale application emoji {} ({}): {error:?}",
                asset.name, existing.id
            );
            mentions.insert(asset.name, existing.to_string());
            continue;
        }

        match create_application_emoji(http, asset).await {
            Ok(emoji) => {
                mentions.insert(asset.name, emoji.to_string());
            }
            Err(error) => {
                warn!(
                    "failed to create application emoji {} from {}: {error:?}",
                    asset.name, asset.file_name
                );
            }
        }
    }

    Ok(MaimaiStatusEmojis { mentions })
}

async fn create_application_emoji(
    http: &serenity::Http,
    asset: EmojiAsset,
) -> Result<serenity::Emoji> {
    #[derive(serde::Serialize)]
    struct CreateEmoji<'a> {
        name: &'a str,
        image: &'a str,
    }

    let image = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(asset.bytes)
    );

    let payload = CreateEmoji {
        name: asset.name,
        image: &image,
    };

    http.create_application_emoji(&payload)
        .await
        .wrap_err_with(|| format!("create application emoji {}", asset.name))
}

pub(crate) fn format_rank(
    emojis: &MaimaiStatusEmojis,
    rank: Option<ScoreRank>,
    missing: &str,
) -> String {
    match rank {
        Some(value) => emojis
            .rank(value)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.as_str().to_string()),
        None => missing.to_string(),
    }
}

pub(crate) fn format_fc(
    emojis: &MaimaiStatusEmojis,
    fc: Option<FcStatus>,
    missing: &str,
) -> String {
    match fc {
        Some(value) => emojis
            .fc(value)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.as_str().to_string()),
        None => missing.to_string(),
    }
}

pub(crate) fn format_sync(
    emojis: &MaimaiStatusEmojis,
    sync: Option<SyncStatus>,
    missing: &str,
) -> String {
    match sync {
        Some(value) => emojis
            .sync(value)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.as_str().to_string()),
        None => missing.to_string(),
    }
}

const fn rank_emoji_name(value: ScoreRank) -> &'static str {
    match value {
        ScoreRank::SssPlus => "maimai_rank_sssp",
        ScoreRank::Sss => "maimai_rank_sss",
        ScoreRank::SsPlus => "maimai_rank_ssp",
        ScoreRank::Ss => "maimai_rank_ss",
        ScoreRank::SPlus => "maimai_rank_sp",
        ScoreRank::S => "maimai_rank_s",
        ScoreRank::Aaa => "maimai_rank_aaa",
        ScoreRank::Aa => "maimai_rank_aa",
        ScoreRank::A => "maimai_rank_a",
        ScoreRank::Bbb => "maimai_rank_bbb",
        ScoreRank::Bb => "maimai_rank_bb",
        ScoreRank::B => "maimai_rank_b",
        ScoreRank::C => "maimai_rank_c",
        ScoreRank::D => "maimai_rank_d",
    }
}

const fn fc_emoji_name(value: FcStatus) -> &'static str {
    match value {
        FcStatus::ApPlus => "maimai_fc_app",
        FcStatus::Ap => "maimai_fc_ap",
        FcStatus::FcPlus => "maimai_fc_fcp",
        FcStatus::Fc => "maimai_fc_fc",
    }
}

const fn sync_emoji_name(value: SyncStatus) -> &'static str {
    match value {
        SyncStatus::FdxPlus => "maimai_sync_fdxp",
        SyncStatus::Fdx => "maimai_sync_fdx",
        SyncStatus::FsPlus => "maimai_sync_fsp",
        SyncStatus::Fs => "maimai_sync_fs",
        SyncStatus::Sync => "maimai_sync_sync",
    }
}

#[cfg(test)]
mod tests {
    use super::{MaimaiStatusEmojis, format_fc, format_rank, format_sync};
    use models::{FcStatus, ScoreRank, SyncStatus};

    #[test]
    fn formatter_falls_back_to_plain_text_without_matching_emoji() {
        let emojis = MaimaiStatusEmojis::default();

        assert_eq!(format_rank(&emojis, Some(ScoreRank::SssPlus), "-"), "SSS+");
        assert_eq!(format_fc(&emojis, Some(FcStatus::ApPlus), "-"), "AP+");
        assert_eq!(format_sync(&emojis, Some(SyncStatus::FdxPlus), "-"), "FDX+");
        assert_eq!(format_rank(&emojis, None, "N/A"), "N/A");
    }

    #[test]
    fn formatter_uses_stored_mentions() {
        let mut emojis = MaimaiStatusEmojis::default();
        emojis
            .mentions
            .insert("maimai_rank_sssp", "<:maimai_rank_sssp:1>".to_string());
        emojis
            .mentions
            .insert("maimai_fc_app", "<:maimai_fc_app:2>".to_string());
        emojis
            .mentions
            .insert("maimai_sync_fdxp", "<:maimai_sync_fdxp:3>".to_string());

        assert_eq!(
            format_rank(&emojis, Some(ScoreRank::SssPlus), "-"),
            "<:maimai_rank_sssp:1>"
        );
        assert_eq!(
            format_fc(&emojis, Some(FcStatus::ApPlus), "-"),
            "<:maimai_fc_app:2>"
        );
        assert_eq!(
            format_sync(&emojis, Some(SyncStatus::FdxPlus), "-"),
            "<:maimai_sync_fdxp:3>"
        );
    }
}
