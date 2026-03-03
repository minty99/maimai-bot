use eyre::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedS3Url {
    pub(crate) bucket: String,
    pub(crate) prefix: String,
}

pub(crate) fn parse_s3_url(raw: &str) -> Result<ParsedS3Url> {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("s3://") else {
        bail!("S3 URL must start with s3://");
    };
    if rest.is_empty() {
        bail!("S3 URL must include a bucket name");
    }

    let mut parts = rest.splitn(2, '/');
    let bucket = parts.next().unwrap_or_default().trim().to_string();
    if bucket.is_empty() {
        bail!("S3 URL must include a bucket name");
    }

    let prefix = parts
        .next()
        .unwrap_or_default()
        .trim_matches('/')
        .to_string();

    Ok(ParsedS3Url { bucket, prefix })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bucket_and_prefix() -> Result<()> {
        let parsed = parse_s3_url("s3://bucket-name/prod/backups/")?;
        assert_eq!(parsed.bucket, "bucket-name");
        assert_eq!(parsed.prefix, "prod/backups");
        Ok(())
    }

    #[test]
    fn parses_bucket_without_prefix() -> Result<()> {
        let parsed = parse_s3_url("s3://bucket-name")?;
        assert_eq!(parsed.bucket, "bucket-name");
        assert!(parsed.prefix.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_invalid_scheme() {
        let err = parse_s3_url("https://bucket-name/prod").unwrap_err();
        assert!(err.to_string().contains("s3://"));
    }

    #[test]
    fn rejects_missing_bucket() {
        let err = parse_s3_url("s3:///prod").unwrap_err();
        assert!(err.to_string().contains("bucket"));
    }
}
