use crate::maimai::models::ChartType;

pub fn song_key_from_title_and_chart(title: &str, chart_type: ChartType) -> eyre::Result<String> {
    let title = title.trim();
    let input = if title.is_empty() { "__empty__" } else { title };
    let chart_prefix = match chart_type {
        ChartType::Std => "STD",
        ChartType::Dx => "DX",
    };
    let material = format!("{chart_prefix}\n{input}");
    Ok(sha256_hex(material.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    hex::encode(digest)
}
