pub mod intl {
    use eyre::WrapErr;
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::Url;

    pub const MAIMAI_MOBILE_ROOT: &str = "https://maimaidx-eng.com/maimai-mobile/";
    pub const RECORD_URL: &str = "https://maimaidx-eng.com/maimai-mobile/record/";

    pub fn default_mobile_headers() -> eyre::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
            ),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            HeaderValue::from_static("en-US,en;q=0.9,ja;q=0.8"),
        );
        Ok(headers)
    }

    pub async fn check_logged_in(client: &reqwest::Client) -> eyre::Result<bool> {
        let resp = client.get(RECORD_URL).send().await.wrap_err("GET record")?;
        let final_url = resp.url().clone();
        let body = resp.text().await.wrap_err("read record html")?;
        Ok(!looks_like_login_or_expired(&final_url, &body))
    }

    pub async fn ensure_logged_in(
        client: &reqwest::Client,
        sega_id: &str,
        sega_password: &str,
    ) -> eyre::Result<()> {
        if check_logged_in(client).await? {
            return Ok(());
        }
        login(client, sega_id, sega_password).await?;
        if !check_logged_in(client).await? {
            return Err(eyre::eyre!("login attempted but still not authenticated"));
        }
        Ok(())
    }

    pub async fn login(
        client: &reqwest::Client,
        sega_id: &str,
        sega_password: &str,
    ) -> eyre::Result<()> {
        let login_page = client
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
            return Ok(());
        }

        let post_url = login_page_url
            .join("/common_auth/login/sid/")
            .wrap_err("resolve login POST url")?;

        let resp = client
            .post(post_url)
            .header(reqwest::header::REFERER, login_page_url.as_str())
            .form(&[
                ("sid", sega_id),
                ("password", sega_password),
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

        Ok(())
    }

    pub fn looks_like_login_or_expired(final_url: &Url, body: &str) -> bool {
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
}
