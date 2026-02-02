#![allow(dead_code)]

use eyre::WrapErr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Copy)]
struct ExtractSpec {
    sheet_name: &'static str,
    data_indexes: &'static [usize],
    data_offsets: [usize; 4],
}

#[derive(Debug, Clone, Copy)]
struct SpreadsheetSpec {
    source_version: i64,
    spreadsheet_id: &'static str,
    extracts: &'static [ExtractSpec],
}

#[derive(Debug, Deserialize)]
struct ValuesResponse {
    #[serde(default)]
    values: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalLevelRow {
    pub song_id: String,
    pub sheet_type: String,
    pub difficulty: String,
    pub internal_level: String,
    pub source_version: i64,
}

const V6_SHEET_ID: &str = "1byBSBQE547KL2KzPkUjY45svcIrJeHh57h-DLJycQbs";
const V7_SHEET_ID: &str = "1xbDMo-36bGL_d435Oy8TTVq4ADFmxl9sYFqhTXiJYRg";
const V8_SHEET_ID: &str = "1xqXfzfDfxiEE9mREwgX_ITIY8AowRM7w-TH2t1I_RJE";
const V9_SHEET_ID: &str = "1vSqx2ghJKjWwCLrDEyZTUMSy5wkq_gY4i0GrJgSreQc";
const V10_SHEET_ID: &str = "1d1AjO92Hj-iay10MsqdR_5TswEaikzC988aEOtFyybo";
const V11_SHEET_ID: &str = "1DKssDl2MM-jjK_GmHPEIVcOMcpVzaeiXA9P5hmhDqAo";
const V12_SHEET_ID: &str = "10N6jmyrzmHrZGbGhDWfpdg4hQKm0t84H2DPkaFG7PNs";
const V13_SHEET_ID: &str = "17vd35oIHxjXPUU-6QJwYoTLPs2nneHN4hokMNLoQQLY";

const V6_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "UNiVERSEPLUS新曲枠",
        data_indexes: &[0, 5, 10, 15, 20],
        data_offsets: [0, 1, 2, 3],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 7, 14],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 7, 14, 21, 28, 35],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 6, 12, 18, 24, 30, 36],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 10, 11, 18],
    },
];

const V7_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "FESTiVAL新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 7, 14],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 7, 14, 21, 28, 35, 42],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[36],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 7, 14, 21, 27, 34, 41],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[48],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V8_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "FESTiVAL+新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 7, 14, 21, 28, 35],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 7, 13, 19, 25, 31],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[37],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 7, 14, 21, 28, 35, 42],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[49],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V9_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "BUDDiES新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21, 28, 35],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 7, 14, 21, 28, 35, 42],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 19, 26, 33],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[39],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 6, 13, 19, 26, 32],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[38],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V10_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "BUDDiES+新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 15, 22, 29, 37],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 8, 15, 22, 29],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 8, 16, 23, 30, 37, 45],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 7, 14, 20, 27],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[34],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 7, 14, 21, 28, 35],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[42],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V11_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "PRiSM新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21, 28],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 8, 15, 22, 29, 36],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 7, 14, 22, 29, 36],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V12_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "PRiSM PLUS新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21, 28],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 6, 12, 18],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 18],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const V13_EXTRACTS: &[ExtractSpec] = &[
    ExtractSpec {
        sheet_name: "CiRCLE新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "PRiSM PLUS新曲",
        data_indexes: &[0, 6, 12, 18, 24],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "新曲枠",
        data_indexes: &[0, 7, 14, 21],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "14以上",
        data_indexes: &[0, 7, 14, 21, 28],
        data_offsets: [0, 2, 3, 5],
    },
    ExtractSpec {
        sheet_name: "13+",
        data_indexes: &[0, 6, 12, 18],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "13",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12+",
        data_indexes: &[0, 6, 12, 18],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "12",
        data_indexes: &[0, 6, 12, 18, 24, 30],
        data_offsets: [0, 1, 2, 4],
    },
    ExtractSpec {
        sheet_name: "Tmai",
        data_indexes: &[0],
        data_offsets: [1, 2, 3, 7],
    },
];

const SPREADSHEETS: &[SpreadsheetSpec] = &[
    SpreadsheetSpec {
        source_version: 6,
        spreadsheet_id: V6_SHEET_ID,
        extracts: V6_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 7,
        spreadsheet_id: V7_SHEET_ID,
        extracts: V7_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 8,
        spreadsheet_id: V8_SHEET_ID,
        extracts: V8_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 9,
        spreadsheet_id: V9_SHEET_ID,
        extracts: V9_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 10,
        spreadsheet_id: V10_SHEET_ID,
        extracts: V10_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 11,
        spreadsheet_id: V11_SHEET_ID,
        extracts: V11_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 12,
        spreadsheet_id: V12_SHEET_ID,
        extracts: V12_EXTRACTS,
    },
    SpreadsheetSpec {
        source_version: 13,
        spreadsheet_id: V13_SHEET_ID,
        extracts: V13_EXTRACTS,
    },
];

fn max_column_for_extract(extract: &ExtractSpec) -> usize {
    let max_data_index = extract.data_indexes.iter().copied().max().unwrap_or(0);
    let max_offset = *extract.data_offsets.iter().max().unwrap_or(&0);
    max_data_index + max_offset
}

async fn fetch_sheet_values(
    client: &reqwest::Client,
    spreadsheet_id: &str,
    sheet_name: &str,
    max_col_idx: usize,
    api_key: &str,
) -> eyre::Result<Vec<Vec<Value>>> {
    const MAX_RETRIES: u32 = 3;
    let end_col = col_idx_to_a1(max_col_idx);
    let range = format!("{sheet_name}!A:{end_col}");
    let encoded_range = urlencoding::encode(&range);
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}/values/{encoded_range}"
    );

    for attempt in 0..MAX_RETRIES {
        match client
            .get(&url)
            .query(&[("key", api_key), ("valueRenderOption", "UNFORMATTED_VALUE")])
            .send()
            .await
        {
            Ok(resp) => match resp.error_for_status() {
                Ok(resp) => match resp.json::<ValuesResponse>().await {
                    Ok(parsed) => return Ok(parsed.values),
                    Err(e) => {
                        if attempt < MAX_RETRIES - 1 {
                            let delay_ms = 500 * 2_u64.pow(attempt);
                            tracing::warn!(
                                "Failed to parse sheet '{}': {}. Retrying in {}ms (attempt {}/{})",
                                sheet_name,
                                e,
                                delay_ms,
                                attempt + 1,
                                MAX_RETRIES
                            );
                            sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        return Err(e).wrap_err("parse sheets values json");
                    }
                },
                Err(e) => {
                    if attempt < MAX_RETRIES - 1 {
                        let delay_ms = 500 * 2_u64.pow(attempt);
                        tracing::warn!(
                            "Sheet '{}' request failed with status: {}. Retrying in {}ms (attempt {}/{})",
                            sheet_name,
                            e,
                            delay_ms,
                            attempt + 1,
                            MAX_RETRIES
                        );
                        sleep(Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                    return Err(e).wrap_err("sheets values status");
                }
            },
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    let delay_ms = 500 * 2_u64.pow(attempt);
                    tracing::warn!(
                        "Connection error for sheet '{}': {}. Retrying in {}ms (attempt {}/{})",
                        sheet_name,
                        e,
                        delay_ms,
                        attempt + 1,
                        MAX_RETRIES
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                return Err(e).wrap_err("GET sheets values");
            }
        }
    }
    unreachable!()
}

fn extract_records_from_values(
    values: &[Vec<Value>],
    spec: &ExtractSpec,
    source_version: i64,
) -> Vec<InternalLevelRow> {
    let mut out = Vec::new();

    for &data_index in spec.data_indexes {
        let title_idx = data_index + spec.data_offsets[0];
        let type_idx = data_index + spec.data_offsets[1];
        let diff_idx = data_index + spec.data_offsets[2];
        let internal_idx = data_index + spec.data_offsets[3];

        for row in values {
            let internal = row.get(internal_idx).and_then(parse_number);
            let Some(internal) = internal.filter(|v| *v > 0.0) else {
                continue;
            };

            let title = row.get(title_idx).and_then(parse_string);
            let sheet_type = row.get(type_idx).and_then(parse_string);
            let difficulty = row.get(diff_idx).and_then(parse_string);

            let Some((song_id, sheet_type, difficulty)) =
                map_row_keys(title, sheet_type, difficulty)
            else {
                continue;
            };

            out.push(InternalLevelRow {
                song_id,
                sheet_type,
                difficulty,
                internal_level: format!("{internal:.1}"),
                source_version,
            });
        }
    }

    out
}

fn map_row_keys(
    title: Option<&str>,
    sheet_type: Option<&str>,
    difficulty: Option<&str>,
) -> Option<(String, String, String)> {
    let title = title?.trim();
    if title.is_empty() {
        return None;
    }

    let song_id = song_id_from_internal_level_title(title)?;

    let sheet_type = match sheet_type?.trim() {
        "STD" => "std",
        "DX" => "dx",
        _ => return None,
    };

    let difficulty = match difficulty?.trim() {
        "EXP" => "expert",
        "MAS" => "master",
        "ReMAS" => "remaster",
        _ => return None,
    };

    Some((song_id, sheet_type.to_string(), difficulty.to_string()))
}

fn parse_string(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn parse_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

pub fn col_idx_to_a1(mut idx: usize) -> String {
    let mut out = Vec::new();
    loop {
        let rem = idx % 26;
        out.push((b'A' + rem as u8) as char);
        if idx < 26 {
            break;
        }
        idx = (idx / 26) - 1;
    }
    out.iter().rev().collect()
}

fn song_id_from_internal_level_title(title: &str) -> Option<String> {
    if title == "Link" {
        return None;
    }

    match manual_mapping(title) {
        ManualMap::Skip => None,
        ManualMap::MapTo(mapped) => Some(mapped.to_string()),
        ManualMap::NoMap => Some(title.to_string()),
    }
}

enum ManualMap {
    Skip,
    MapTo(&'static str),
    NoMap,
}

fn manual_mapping(title: &str) -> ManualMap {
    match title {
        "ATLUS RUSH" => ManualMap::MapTo("ATLAS RUSH"),
        "Agitation!" => ManualMap::MapTo("Agitation！"),
        "Alea jacta est" => ManualMap::MapTo("Alea jacta est!"),
        "Baban!!  ー甘い罠ー" => ManualMap::MapTo("BaBan!! －甘い罠－"),
        "Backyun! -悪い女-" => ManualMap::MapTo("Backyun! －悪い女－"),
        "Bad Apple!! feat nomico" => ManualMap::MapTo("Bad Apple!! feat.nomico"),
        "Bad Apple!! feat.nomico 〜五十嵐撫子Ver.〜" => {
            ManualMap::MapTo("Bad Apple!! feat.nomico ～五十嵐 撫子 Ver.～")
        }
        "Bad Apple!! feat.nomico(REDALiCE Remix)" => {
            ManualMap::MapTo("Bad Apple!! feat.nomico (REDALiCE Remix)")
        }
        "Boys O'Clock" => ManualMap::MapTo("Boys O’Clock"),
        "Caliburne ～Story of the Legendary Sword～" => {
            ManualMap::MapTo("Caliburne ～Story of the Legendary sword～")
        }
        "Change Our MIRAI!" => ManualMap::MapTo("Change Our MIRAI！"),
        "City Escape:Act1" => ManualMap::MapTo("City Escape: Act1"),
        "Cyber Sparks" => ManualMap::MapTo("CYBER Sparks"),
        "D✪N’T ST✪P R✪CKIN’" => ManualMap::MapTo("D✪N’T  ST✪P  R✪CKIN’"),
        "Excalibur ～Revived Resolution～" => {
            ManualMap::MapTo("Excalibur ～Revived resolution～")
        }
        "FREEDOM DiVE(tpz Overcute Remix)" => ManualMap::MapTo("FREEDOM DiVE (tpz Overcute Remix)"),
        "GRANDIR" => ManualMap::MapTo("GRÄNDIR"),
        "God Knows…" => ManualMap::MapTo("God knows..."),
        "Good Bye, Merry-Go-Round." => ManualMap::MapTo("Good bye, Merry-Go-Round."),
        "Got more raves?" => ManualMap::MapTo("Got more raves？"),
        "Help me, ERINNNNNN!! （Band ver.）" => {
            ManualMap::MapTo("Help me, ERINNNNNN!!（Band ver.）")
        }
        "Imperishable Night 2006(2016 Refine)" => {
            ManualMap::MapTo("Imperishable Night 2006 (2016 Refine)")
        }
        "Jack-the-Ripper♦" => ManualMap::MapTo("Jack-the-Ripper◆"),
        "Jorqer" => ManualMap::MapTo("Jörqer"),
        "ΚΗΥΜΞΧΛ\u{202C}" => ManualMap::MapTo("KHYMΞXΛ"),
        "L4TS:2018 (feat. あひる ＆ KTA)" => ManualMap::MapTo("L4TS:2018 (feat. あひる & KTA)"),
        "L4TS:2018(feat.あひる＆KTA)" => ManualMap::MapTo("L4TS:2018 (feat. あひる & KTA)"),
        "L'epilogue" => ManualMap::MapTo("L'épilogue"),
        "Love kills U" => ManualMap::MapTo("Love Kills U"),
        "Love’s Theme of BADASS ～バッド・アス 愛のテーマ～" => {
            ManualMap::MapTo("Love's Theme of BADASS ～バッド・アス 愛のテーマ～")
        }
        "Melody!" => ManualMap::MapTo("Melody！"),
        "Mjolnir" => ManualMap::MapTo("Mjölnir"),
        "Party 4U \"holy nite mix\"" => ManualMap::MapTo("Party 4U ”holy nite mix”"),
        "Quartet Theme[Reborn]" => ManualMap::MapTo("Quartet Theme [Reborn]"),
        "REVIVER オルタンシア･サーガ-蒼の騎士団- オリジナルVer." => {
            ManualMap::MapTo("REVIVER オルタンシア・サーガ -蒼の騎士団- オリジナルVer.")
        }
        "Re:End of a Dream" => ManualMap::MapTo("Re：End of a Dream"),
        "Retribution 〜 Cycle of Redemption 〜" => {
            ManualMap::MapTo("Retribution ～ Cycle of Redemption ～")
        }
        "Rooftop Run: Act１" => ManualMap::MapTo("Rooftop Run: Act1"),
        "Rooftop Run:Act1" => ManualMap::MapTo("Rooftop Run: Act1"),
        "R’N’R Monsta" => ManualMap::MapTo("R'N'R Monsta"),
        "SQUAD -Phvntom-" => ManualMap::MapTo("SQUAD-Phvntom-"),
        "Save This World νMix" => ManualMap::MapTo("Save This World νMIX"),
        "Seclet Sleuth" => ManualMap::MapTo("Secret Sleuth"),
        "Session High↑" => ManualMap::MapTo("Session High⤴"),
        "Seyana.～何でも言うことをきいてくれるアカネチャン～" => {
            ManualMap::MapTo("Seyana. ～何でも言うことを聞いてくれるアカネチャン～")
        }
        "Seyana.～何でも言うことを聞いてくれるアカネチャン～" => {
            ManualMap::MapTo("Seyana. ～何でも言うことを聞いてくれるアカネチャン～")
        }
        "Sky High[Reborn]" => ManualMap::MapTo("Sky High [Reborn]"),
        "Sqlupp(Camellia's Sqleipd*Hiytex Remix)" => {
            ManualMap::MapTo("Sqlupp (Camellia's \"Sqleipd*Hiytex\" Remix)")
        }
        "Sweetie×2" => ManualMap::MapTo("Sweetiex2"),
        "System \"Z\"" => ManualMap::MapTo("System “Z”"),
        "Tic Tac DREAMIN'" => ManualMap::MapTo("Tic Tac DREAMIN’"),
        "Turn Around" => ManualMap::MapTo("Turn around"),
        "Urban Crusher[Remix]" => ManualMap::MapTo("Urban Crusher [Remix]"),
        "YA･DA･YO[Reborn]" => ManualMap::MapTo("YA･DA･YO [Reborn]"),
        "Yakumo>>JOINT STRUGGLE(2019 update)" => {
            ManualMap::MapTo("Yakumo >>JOINT STRUGGLE (2019 Update)")
        }
        "falling" => ManualMap::MapTo("Falling"),
        "null" => ManualMap::MapTo("　"),
        "Åntinomiε" => ManualMap::MapTo("Åntinomiε"),
        "“411Ψ892”" => ManualMap::MapTo("\"411Ψ892\""),
        "≠彡゛/了→" => ManualMap::MapTo("≠彡\"/了→"),
        "【東方ニコカラ】秘神マターラfeat.魂音泉【IOSYS】" => {
            ManualMap::MapTo("【東方ニコカラ】秘神マターラ feat.魂音泉【IOSYS】")
        }
        "ずんだもんの朝食　～目覚ましずんラップ～" => {
            ManualMap::MapTo("ずんだもんの朝食　〜目覚ましずんラップ〜")
        }
        "なだめスかし Negotiation(TVsize)" => {
            ManualMap::MapTo("なだめスかし Negotiation（TVsize）")
        }
        "はげしこの夜-Psylent Crazy Night-" => {
            ManualMap::MapTo("はげしこの夜 -Psylent Crazy Night-")
        }
        "ぼくたちいつでもしゅわっしゅわ！" => {
            ManualMap::MapTo("ぼくたちいつでも　しゅわっしゅわ！")
        }
        "ウッーウッーウマウマ( ﾟ∀ﾟ)" => ManualMap::MapTo("ウッーウッーウマウマ(ﾟ∀ﾟ)"),
        "オパ！オパ！RACER -GMT mashup-" => {
            ManualMap::MapTo("オパ! オパ! RACER -GMT mashup-")
        }
        "オーケー？オーライ！" => ManualMap::MapTo("オーケー？　オーライ！"),
        "ガチャガチャきゅ～と・ふぃぎゅ＠メイト" => {
            ManualMap::MapTo("ガチャガチャきゅ～と・ふぃぎゅ@メイト")
        }
        "スカーレット警察のゲットーパトロール２４時" => {
            ManualMap::MapTo("スカーレット警察のゲットーパトロール24時")
        }
        "チルノのパーフェクトさんすう教室 ⑨周年バージョン" => {
            ManualMap::MapTo("チルノのパーフェクトさんすう教室　⑨周年バージョン")
        }
        "トルコ行進曲 -オワタ＼(^o^)／" => {
            ManualMap::MapTo("トルコ行進曲 - オワタ＼(^o^)／")
        }
        "ナイト・オブ・ナイツ(Cranky Remix)" => {
            ManualMap::MapTo("ナイト・オブ・ナイツ (Cranky Remix)")
        }
        "ファンタジーゾーンOPA!-OPA! -GMT remix-" => {
            ManualMap::MapTo("ファンタジーゾーン OPA-OPA! -GMT remix-")
        }
        "プラネタリウム・レビュー" => ManualMap::MapTo("プラネタリウム・レヴュー"),
        "レッツゴー！陰陽師" => ManualMap::MapTo("レッツゴー!陰陽師"),
        "夜明けまであと3秒" => ManualMap::MapTo("夜明けまであと３秒"),
        "天狗の落とし文 feat.ｙｔｒ" => ManualMap::MapTo("天狗の落とし文 feat. ｙｔｒ"),
        "好きな総菜発表ドラゴン" => ManualMap::MapTo("好きな惣菜発表ドラゴン"),
        "教えて!!魔法のLyric" => ManualMap::MapTo("教えて!! 魔法のLyric"),
        "曖昧Mind" => ManualMap::MapTo("曖昧mind"),
        "泣き虫O'Clock" => ManualMap::MapTo("泣き虫O'clock"),
        "砂の惑星 feat.HATSUNE MIKU" => ManualMap::MapTo("砂の惑星 feat. HATSUNE MIKU"),
        "管弦楽組曲 第3番 ニ長調「第2曲(G線上のアリア)」BWV.1068-2" => {
            ManualMap::MapTo("管弦楽組曲 第3番 ニ長調「第2曲（G線上のアリア）」BWV.1068-2")
        }
        "紅星ミゼラブル〜廃憶編" => ManualMap::MapTo("紅星ミゼラブル～廃憶編"),
        "赤心性:カマトト荒療治" => ManualMap::MapTo("赤心性：カマトト荒療治"),
        "超熊猫的周遊記(ワンダーパンダートラベラー)" => {
            ManualMap::MapTo("超熊猫的周遊記（ワンダーパンダートラベラー）")
        }
        "雷切 -RAIKIRI-" => ManualMap::MapTo("雷切-RAIKIRI-"),
        "(  Ꙭ)ﾌﾞｯｺﾛﾘ食べよう" => ManualMap::Skip,
        "test" => ManualMap::Skip,
        "実験" => ManualMap::Skip,
        _ => ManualMap::NoMap,
    }
}

pub async fn fetch_internal_levels(
    client: &reqwest::Client,
    google_api_key: &str,
) -> eyre::Result<HashMap<(String, String, String), InternalLevelRow>> {
    let mut all_rows = Vec::new();
    let mut failed_sheets = Vec::new();
    let mut total_sheets = 0;

    for spreadsheet in SPREADSHEETS {
        for extract in spreadsheet.extracts {
            total_sheets += 1;
            let sheet_identifier = format!(
                "v{} / {}",
                spreadsheet.source_version, extract.sheet_name
            );

            match fetch_sheet_values(
                client,
                spreadsheet.spreadsheet_id,
                extract.sheet_name,
                max_column_for_extract(extract),
                google_api_key,
            )
            .await
            {
                Ok(values) => {
                    all_rows.extend(extract_records_from_values(
                        &values,
                        extract,
                        spreadsheet.source_version,
                    ));
                }
                Err(e) => {
                    tracing::error!("Failed to fetch sheet '{}': {:#}", sheet_identifier, e);
                    failed_sheets.push(sheet_identifier);
                }
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    let mut result = HashMap::new();
    for row in all_rows {
        let key = (
            row.song_id.clone(),
            row.sheet_type.clone(),
            row.difficulty.clone(),
        );
        result
            .entry(key)
            .and_modify(|existing: &mut InternalLevelRow| {
                if row.source_version > existing.source_version {
                    *existing = row.clone();
                }
            })
            .or_insert(row);
    }

    let success_count = total_sheets - failed_sheets.len();
    tracing::info!(
        "Internal levels: fetched {} / {} sheets successfully",
        success_count,
        total_sheets
    );

    if !failed_sheets.is_empty() {
        tracing::warn!(
            "Failed to fetch {} sheets: {}",
            failed_sheets.len(),
            failed_sheets.join(", ")
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_idx_to_a1_works() {
        assert_eq!(col_idx_to_a1(0), "A");
        assert_eq!(col_idx_to_a1(25), "Z");
        assert_eq!(col_idx_to_a1(26), "AA");
        assert_eq!(col_idx_to_a1(27), "AB");
        assert_eq!(col_idx_to_a1(51), "AZ");
        assert_eq!(col_idx_to_a1(52), "BA");
    }

    #[test]
    fn extract_records_from_values_parses_numeric_internal_level() {
        let spec = ExtractSpec {
            sheet_name: "dummy",
            data_indexes: &[0],
            data_offsets: [0, 1, 2, 3],
        };

        let values = vec![vec![
            Value::String("Some Song".to_string()),
            Value::String("STD".to_string()),
            Value::String("MAS".to_string()),
            Value::Number(serde_json::Number::from_f64(13.7).unwrap()),
        ]];

        let rows = extract_records_from_values(&values, &spec, 13);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].song_id, "Some Song");
        assert_eq!(rows[0].sheet_type, "std");
        assert_eq!(rows[0].difficulty, "master");
        assert_eq!(rows[0].internal_level, "13.7");
        assert_eq!(rows[0].source_version, 13);
    }
}
