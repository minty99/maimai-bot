use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;

use eyre::WrapErr;
use reqwest::Url;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use time::OffsetDateTime;

use maimai_auth::intl;
use models::config::AppConfig;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct HtmlResponse {
    pub(crate) final_url: Url,
    pub(crate) html: String,
}

#[derive(Debug, Clone)]
pub(crate) struct MaimaiClient {
    config: AppConfig,
    cookie_store: Arc<CookieStoreMutex>,
    client: Arc<reqwest::Client>,
}

impl MaimaiClient {
    pub(crate) fn new(config: &AppConfig) -> eyre::Result<Self> {
        let cookie_store = load_cookie_store(&config.cookie_path).wrap_err("load cookie store")?;
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        let client = Arc::new(
            reqwest::Client::builder()
                .default_headers(intl::default_mobile_headers()?)
                .redirect(reqwest::redirect::Policy::limited(10))
                .cookie_provider(cookie_store.clone())
                .build()
                .wrap_err("build reqwest client")?,
        );

        Ok(Self {
            config: config.clone(),
            cookie_store,
            client,
        })
    }

    pub(crate) async fn check_logged_in(&mut self) -> eyre::Result<bool> {
        ensure_not_maintenance_now()?;
        intl::check_logged_in(self.client.as_ref()).await
    }

    pub(crate) async fn ensure_logged_in(&mut self) -> eyre::Result<()> {
        ensure_not_maintenance_now()?;
        if self.check_logged_in().await? {
            return Ok(());
        }
        self.login().await?;
        if !self.check_logged_in().await? {
            return Err(eyre::eyre!("login attempted but still not authenticated"));
        }
        save_cookie_store(&self.config.cookie_path, &self.cookie_store)
            .wrap_err("save cookie store")?;
        Ok(())
    }

    pub(crate) async fn login(&mut self) -> eyre::Result<()> {
        ensure_not_maintenance_now()?;
        intl::login(
            self.client.as_ref(),
            &self.config.sega_id,
            &self.config.sega_password,
        )
        .await?;

        save_cookie_store(&self.config.cookie_path, &self.cookie_store)
            .wrap_err("save cookie store")?;
        Ok(())
    }

    pub(crate) async fn get_bytes(&self, url: &Url) -> eyre::Result<Vec<u8>> {
        ensure_not_maintenance_now()?;
        let resp = self
            .client
            .as_ref()
            .get(url.clone())
            .send()
            .await
            .wrap_err("GET")?;
        let status = resp.status();
        let final_url = resp.url().clone();
        let bytes = resp.bytes().await.wrap_err("read response bytes")?;
        if !status.is_success() {
            if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
                return Err(eyre::eyre!(
                    "site unavailable (503). maimai DX NET may be under maintenance. url={final_url}"
                ));
            }
            return Err(eyre::eyre!("non-success status: {status} url={final_url}"));
        }
        Ok(bytes.to_vec())
    }

    #[allow(dead_code)]
    pub(crate) async fn get_html_response(&self, url: &Url) -> eyre::Result<HtmlResponse> {
        ensure_not_maintenance_now()?;
        let resp = self
            .client
            .as_ref()
            .get(url.clone())
            .send()
            .await
            .wrap_err("GET")?;
        let status = resp.status();
        let final_url = resp.url().clone();
        let html = resp.text().await.wrap_err("read response text")?;
        if !status.is_success() {
            if status == reqwest::StatusCode::SERVICE_UNAVAILABLE {
                return Err(eyre::eyre!(
                    "site unavailable (503). maimai DX NET may be under maintenance. url={final_url}"
                ));
            }
            return Err(eyre::eyre!("non-success status: {status} url={final_url}"));
        }

        Ok(HtmlResponse { final_url, html })
    }
}

pub(crate) fn is_maintenance_window_now() -> bool {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    is_maintenance_window_hour(now.hour())
}

fn is_maintenance_window_hour(hour: u8) -> bool {
    (4..7).contains(&hour)
}

fn ensure_not_maintenance_now() -> eyre::Result<()> {
    if is_maintenance_window_now() {
        return Err(eyre::eyre!(
            "maimai DX NET maintenance window (04:00-07:00 local time); skipping request"
        ));
    }
    Ok(())
}

fn load_cookie_store(path: &std::path::Path) -> eyre::Result<CookieStore> {
    if !path.exists() {
        return Ok(CookieStore::default());
    }
    let file = File::open(path).wrap_err("open cookie file")?;
    let reader = BufReader::new(file);
    cookie_store::serde::json::load_all(reader).map_err(|e| eyre::eyre!("parse cookie json: {e}"))
}

fn save_cookie_store(
    path: &std::path::Path,
    cookie_store: &Arc<CookieStoreMutex>,
) -> eyre::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).wrap_err("create cookie directory")?;
    }

    let file = File::create(path).wrap_err("create cookie file")?;
    let mut writer = BufWriter::new(file);
    let guard = cookie_store
        .lock()
        .map_err(|_| eyre::eyre!("cookie store mutex poisoned"))?;
    cookie_store::serde::json::save_incl_expired_and_nonpersistent(&guard, &mut writer)
        .map_err(|e| eyre::eyre!("write cookie json: {e}"))?;
    Ok(())
}
