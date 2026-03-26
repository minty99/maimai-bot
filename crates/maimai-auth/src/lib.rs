pub mod intl {
    use eyre::WrapErr;
    use reqwest::Url;
    use reqwest::header::{HeaderMap, HeaderValue};

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

        let post_url = extract_login_post_url(&login_page_url, &login_page_html)
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
            .wrap_err("POST SEGA ID login")?
            .error_for_status()
            .wrap_err("SEGA ID login returned non-success status")?;

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

    fn extract_login_post_url(login_page_url: &Url, login_page_html: &str) -> eyre::Result<Url> {
        if let Some(action) = find_form_action(login_page_html, "sidForm") {
            return login_page_url
                .join(&action)
                .wrap_err("join sidForm action with login page url");
        }

        login_page_url
            .join("/common_auth/login/sid")
            .wrap_err("fallback to known SEGA ID login path")
    }

    fn find_form_action(html: &str, form_id: &str) -> Option<String> {
        let form_id_attr = format!("id=\"{form_id}\"");
        let id_idx = html.find(&form_id_attr)?;
        let form_idx = html[..id_idx].rfind("<form")?;
        let tag_end = html[id_idx..].find('>')? + id_idx;
        let tag = &html[form_idx..=tag_end];

        extract_attr_value(tag, "action")
    }

    fn extract_attr_value(tag: &str, attr_name: &str) -> Option<String> {
        for quote in ['"', '\''] {
            let needle = format!("{attr_name}={quote}");
            if let Some(attr_idx) = tag.find(&needle) {
                let start = attr_idx + needle.len();
                let end = tag[start..].find(quote)? + start;
                return Some(tag[start..end].to_string());
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use reqwest::Url;

        use super::{
            extract_attr_value, extract_login_post_url, find_form_action,
            looks_like_login_or_expired,
        };

        #[test]
        fn extracts_sid_form_action_from_login_page() {
            let html = r#"
                <html>
                    <body>
                        <form action="/common_auth/login/sid" method="post" id="sidForm">
                            <input name="sid" />
                        </form>
                    </body>
                </html>
            "#;

            assert_eq!(
                find_form_action(html, "sidForm").as_deref(),
                Some("/common_auth/login/sid")
            );
        }

        #[test]
        fn resolves_login_post_url_without_trailing_slash() {
            let html = r#"
                <form action="/common_auth/login/sid" method="post" id="sidForm"></form>
            "#;
            let login_page_url = Url::parse(
                "https://lng-tgk-aime-gw.am-all.net/common_auth/login?site_id=maimaidxex",
            )
            .expect("valid url");

            let post_url = extract_login_post_url(&login_page_url, html).expect("post url");

            assert_eq!(
                post_url.as_str(),
                "https://lng-tgk-aime-gw.am-all.net/common_auth/login/sid"
            );
        }

        #[test]
        fn falls_back_to_known_login_path_when_action_missing() {
            let login_page_url = Url::parse(
                "https://lng-tgk-aime-gw.am-all.net/common_auth/login?site_id=maimaidxex",
            )
            .expect("valid url");

            let post_url = extract_login_post_url(&login_page_url, "<form id=\"sidForm\"></form>")
                .expect("post url");

            assert_eq!(
                post_url.as_str(),
                "https://lng-tgk-aime-gw.am-all.net/common_auth/login/sid"
            );
        }

        #[test]
        fn extracts_single_quoted_attribute_values() {
            let tag = "<form action='/common_auth/login/sid' id='sidForm'>";
            assert_eq!(
                extract_attr_value(tag, "action").as_deref(),
                Some("/common_auth/login/sid")
            );
        }

        #[test]
        fn detects_error_page_as_unauthenticated() {
            let url =
                Url::parse("https://maimaidx-eng.com/maimai-mobile/error/").expect("valid url");
            let body = "<title>ERROR CODE: 500</title>The connection time has been expired";

            assert!(looks_like_login_or_expired(&url, body));
        }
    }
}
