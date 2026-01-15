pub fn song_key_from_title(title: &str) -> eyre::Result<String> {
    let title = title.trim();
    let input = if title.is_empty() { "__empty__" } else { title };
    Ok(sha256_hex(input.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    hex::encode(digest)
}
