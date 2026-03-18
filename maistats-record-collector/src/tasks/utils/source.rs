use std::collections::BTreeMap;
use std::path::Path;

use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

use crate::http_client::MaimaiClient;
use crate::tasks::utils::player::fetch_player_data_logged_in;
use crate::tasks::utils::playlog_detail::fetch_playlog_detail;
use crate::tasks::utils::recent::fetch_recent_entries_logged_in;
use crate::tasks::utils::scores::fetch_score_entries_logged_in;
use crate::tasks::utils::song_detail::fetch_song_detail_by_idx;
use models::{
    ParsedPlayRecord, ParsedPlayerProfile, ParsedPlaylogDetail, ParsedScoreEntry, ParsedSongDetail,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpectedPage {
    PlayerData,
    Recent,
    ScoresList { diff: u8 },
    PlaylogDetail { idx: String },
    MusicDetail { idx: String },
}

#[allow(async_fn_in_trait)]
pub trait CollectorSource {
    async fn ensure_session(&mut self) -> Result<()>;

    async fn fetch_player_data(&mut self) -> Result<ParsedPlayerProfile>;

    async fn fetch_recent_entries(&mut self) -> Result<Vec<ParsedPlayRecord>>;

    async fn fetch_score_entries(&mut self, diff: u8) -> Result<Vec<ParsedScoreEntry>>;

    async fn fetch_playlog_detail(&mut self, idx: &str) -> Result<ParsedPlaylogDetail>;

    async fn fetch_song_detail(&mut self, idx: &str) -> Result<ParsedSongDetail>;
}

impl CollectorSource for MaimaiClient {
    async fn ensure_session(&mut self) -> Result<()> {
        self.ensure_logged_in().await.wrap_err("ensure logged in")
    }

    async fn fetch_player_data(&mut self) -> Result<ParsedPlayerProfile> {
        fetch_player_data_logged_in(self).await
    }

    async fn fetch_recent_entries(&mut self) -> Result<Vec<ParsedPlayRecord>> {
        fetch_recent_entries_logged_in(self).await
    }

    async fn fetch_score_entries(&mut self, diff: u8) -> Result<Vec<ParsedScoreEntry>> {
        fetch_score_entries_logged_in(self, diff).await
    }

    async fn fetch_playlog_detail(&mut self, idx: &str) -> Result<ParsedPlaylogDetail> {
        fetch_playlog_detail(self, idx).await
    }

    async fn fetch_song_detail(&mut self, idx: &str) -> Result<ParsedSongDetail> {
        fetch_song_detail_by_idx(self, idx).await
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixtureScoreLists {
    #[serde(default)]
    pub diff0: Vec<ParsedScoreEntry>,
    #[serde(default)]
    pub diff1: Vec<ParsedScoreEntry>,
    #[serde(default)]
    pub diff2: Vec<ParsedScoreEntry>,
    #[serde(default)]
    pub diff3: Vec<ParsedScoreEntry>,
    #[serde(default)]
    pub diff4: Vec<ParsedScoreEntry>,
}

impl FixtureScoreLists {
    fn for_diff(&self, diff: u8) -> Result<Vec<ParsedScoreEntry>> {
        let entries = match diff {
            0 => &self.diff0,
            1 => &self.diff1,
            2 => &self.diff2,
            3 => &self.diff3,
            4 => &self.diff4,
            _ => return Err(eyre::eyre!("fixture score diff must be 0..4")),
        };
        Ok(entries.clone())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixtureCollectorData {
    pub player_data: Option<ParsedPlayerProfile>,
    pub recent_entries: Option<Vec<ParsedPlayRecord>>,
    #[serde(default)]
    pub score_lists: FixtureScoreLists,
    #[serde(default)]
    pub playlog_details: BTreeMap<String, ParsedPlaylogDetail>,
    #[serde(default)]
    pub song_details: BTreeMap<String, ParsedSongDetail>,
}

#[derive(Debug, Clone, Default)]
pub struct FixtureCollectorSource {
    data: FixtureCollectorData,
    fetch_log: Vec<ExpectedPage>,
}

impl FixtureCollectorSource {
    pub fn from_data(data: FixtureCollectorData) -> Self {
        Self {
            data,
            fetch_log: Vec::new(),
        }
    }

    pub fn from_fixture_dir(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path.join("source.json"))
            .wrap_err("read fixture collector source json")?;
        let data: FixtureCollectorData =
            serde_json::from_str(&raw).wrap_err("parse fixture collector source json")?;
        Ok(Self::from_data(data))
    }

    pub fn fetch_log(&self) -> &[ExpectedPage] {
        &self.fetch_log
    }

    fn log_fetch(&mut self, page: ExpectedPage) {
        self.fetch_log.push(page);
    }

    fn synthesize_playlog_detail(&self, idx: &str) -> Option<ParsedPlaylogDetail> {
        let recent_title = self
            .data
            .recent_entries
            .as_ref()?
            .iter()
            .find(|entry| entry.playlog_detail_idx.as_deref() == Some(idx))?
            .title
            .clone();
        let music_detail_idx = idx
            .split_once("::")
            .map(|(music_detail_idx, _)| music_detail_idx)
            .unwrap_or(idx);
        Some(ParsedPlaylogDetail {
            title: recent_title,
            music_detail_idx: music_detail_idx.to_string(),
        })
    }
}

impl CollectorSource for FixtureCollectorSource {
    async fn ensure_session(&mut self) -> Result<()> {
        Ok(())
    }

    async fn fetch_player_data(&mut self) -> Result<ParsedPlayerProfile> {
        self.log_fetch(ExpectedPage::PlayerData);
        self.data
            .player_data
            .clone()
            .ok_or_else(|| eyre::eyre!("fixture is missing player_data"))
    }

    async fn fetch_recent_entries(&mut self) -> Result<Vec<ParsedPlayRecord>> {
        self.log_fetch(ExpectedPage::Recent);
        self.data
            .recent_entries
            .clone()
            .ok_or_else(|| eyre::eyre!("fixture is missing recent_entries"))
    }

    async fn fetch_score_entries(&mut self, diff: u8) -> Result<Vec<ParsedScoreEntry>> {
        self.log_fetch(ExpectedPage::ScoresList { diff });
        self.data.score_lists.for_diff(diff)
    }

    async fn fetch_playlog_detail(&mut self, idx: &str) -> Result<ParsedPlaylogDetail> {
        self.log_fetch(ExpectedPage::PlaylogDetail {
            idx: idx.to_string(),
        });
        self.data
            .playlog_details
            .get(idx)
            .cloned()
            .or_else(|| self.synthesize_playlog_detail(idx))
            .ok_or_else(|| eyre::eyre!("fixture is missing playlog_detail idx={idx}"))
    }

    async fn fetch_song_detail(&mut self, idx: &str) -> Result<ParsedSongDetail> {
        self.log_fetch(ExpectedPage::MusicDetail {
            idx: idx.to_string(),
        });
        self.data
            .song_details
            .get(idx)
            .cloned()
            .ok_or_else(|| eyre::eyre!("fixture is missing music_detail idx={idx}"))
    }
}
