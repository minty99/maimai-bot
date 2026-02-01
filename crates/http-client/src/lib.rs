use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;

use eyre::WrapErr;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use time::OffsetDateTime;

use models::config::AppConfig;

const MAIMAI_MOBILE_ROOT: &str = "https://maimaidx-eng.com/maimai-mobile/";
const RECORD_URL: &str = "https://maimaidx-eng.com/maimai-mobile/record/";

#[derive(Debug, Clone)]
pub struct MaimaiClient {
    config: AppConfig,
    cookie_store: Arc<CookieStoreMutex>,
    client: Arc<reqwest::Client>,
}

impl MaimaiClient {
    pub fn new(config: &AppConfig) -> eyre::Result<Self> {
        let cookie_store = load_cookie_store(&config.cookie_path).wrap_err("load cookie store")?;
        let cookie_store = Arc::new(CookieStoreMutex::new(cookie_store));

        let client = Arc::new(
            reqwest::Client::builder()
                .default_headers(default_headers()?)
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

    pub async fn check_logged_in(&mut self) -> eyre::Result<bool> {
        ensure_not_maintenance_now()?;
        let resp = self
            .client
            .as_ref()
            .get(RECORD_URL)
            .send()
            .await
            .wrap_err("GET record")?;
        let final_url = resp.url().clone();
        let body = resp.text().await.wrap_err("read record html")?;
        Ok(!looks_like_login_or_expired(&final_url, &body))
    }

    pub async fn ensure_logged_in(&mut self) -> eyre::Result<()> {
        ensure_not_maintenance_now()?;
        if self.check_logged_in().await? {
            return Ok(());
        }
        self.login().await?;
        if !self.check_logged_in().await? {
            return Err(eyre::eyre!("login attempted but still not authenticated"));
        }
        Ok(())
    }

    pub async fn login(&mut self) -> eyre::Result<()> {
        ensure_not_maintenance_now()?;
        let login_page = self
            .client
            .as_ref()
            .get(MAIMAI_MOBILE_ROOT)
            .send()
            .await
            .wrap_err("GET maimai mobile root (redirects to login)")?;

        let login_page_url = login_page.url().clone();
        let login_page_html = login_page.text().await.wrap_err("read login page html")?;

        if !login_page_html.contains("id=\"sidForm\"") {
            if looks_like_login_or_expired(&login_page_url, &login_page_html) {
                return Err(eyre::eyre!(
                    "unexpected login response. final_url={}",
                    login_page_url
                ));
            }
            save_cookie_store(&self.config.cookie_path, &self.cookie_store)
                .wrap_err("save cookie store")?;
            return Ok(());
        }

        let post_url = login_page_url
            .join("/common_auth/login/sid/")
            .wrap_err("resolve login POST url")?;

        let resp = self
            .client
            .as_ref()
            .post(post_url)
            .header(reqwest::header::REFERER, login_page_url.as_str())
            .form(&[
                ("sid", self.config.sega_id.as_str()),
                ("password", self.config.sega_password.as_str()),
                ("retention", "1"),
            ])
            .send()
            .await
            .wrap_err("POST SEGA ID login")?;

        let final_url = resp.url().clone();
        let body = resp.text().await.wrap_err("read post-login response")?;

        if looks_like_login_or_expired(&final_url, &body) {
            return Err(eyre::eyre!(
                "login failed or not completed. final_url={}",
                final_url
            ));
        }

        save_cookie_store(&self.config.cookie_path, &self.cookie_store)
            .wrap_err("save cookie store")?;
        Ok(())
    }

    pub async fn get_bytes(&self, url: &Url) -> eyre::Result<Vec<u8>> {
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
}

pub fn is_maintenance_window_now() -> bool {
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

fn default_headers() -> eyre::Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
        ),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"),
    );
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.9,ja;q=0.8"),
    );
    Ok(headers)
}

fn looks_like_login_or_expired(final_url: &Url, body: &str) -> bool {
    let url_str = final_url.as_str();
    if final_url.path().starts_with("/maimai-mobile/error/") {
        return true;
    }
    if url_str.contains("/common_auth/login") {
        return true;
    }
    if final_url
        .domain()
        .is_some_and(|d| d.ends_with("am-all.net"))
        && url_str.contains("/common_auth/")
    {
        return true;
    }
    if body.contains("Please login again.") {
        return true;
    }
    if body.contains("ERROR CODE") || body.contains("title_error.png") {
        return true;
    }
    if body.contains("The connection time has been expired") {
        return true;
    }
    false
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
