use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client, primitives::ByteStream};
use eyre::{Result, WrapErr};

use crate::config::BackupConfig;

#[derive(Clone)]
pub(crate) struct S3Uploader {
    bucket: String,
    prefix: String,
    client: Client,
}

impl S3Uploader {
    pub(crate) async fn from_config(config: &BackupConfig) -> Result<Self> {
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new(config.region.clone()))
            .load()
            .await;
        let client = Client::new(&shared_config);

        Ok(Self {
            bucket: config.s3_bucket.clone(),
            prefix: config.s3_prefix.clone(),
            client,
        })
    }

    pub(crate) async fn upload_snapshot(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .wrap_err("upload sqlite backup snapshot to S3")?;
        Ok(())
    }

    pub(crate) fn prefixed_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix, key)
        }
    }
}
