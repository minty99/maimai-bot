use ab_glyph::FontArc;
use eyre::{Result, WrapErr};
use image::imageops::{FilterType, overlay, resize};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use crate::client::SongInfoClient;
use crate::embeds::format_level_with_internal;

const CANVAS_BG: Rgba<u8> = Rgba([16, 20, 24, 255]);
const HEADER_BG: Rgba<u8> = Rgba([27, 36, 46, 255]);
const CARD_BG: Rgba<u8> = Rgba([245, 248, 252, 255]);
const CARD_BORDER: Rgba<u8> = Rgba([204, 212, 224, 255]);
const PLACEHOLDER_BG: Rgba<u8> = Rgba([224, 232, 242, 255]);
const TEXT_PRIMARY_DARK: Rgba<u8> = Rgba([28, 35, 45, 255]);
const TEXT_SECONDARY_DARK: Rgba<u8> = Rgba([72, 84, 102, 255]);
const TEXT_LIGHT: Rgba<u8> = Rgba([233, 240, 247, 255]);
const TEXT_ACCENT: Rgba<u8> = Rgba([24, 114, 188, 255]);

const MARGIN: u32 = 24;
const HEADER_H: u32 = 96;
const COLS: u32 = 5;
const CARD_W: u32 = 380;
const CARD_H: u32 = 172;
const GAP_X: u32 = 16;
const GAP_Y: u32 = 12;
const COVER_SIZE: u32 = 92;

#[derive(Debug, Clone)]
pub(crate) struct RatingImageEntry {
    pub(crate) title: String,
    pub(crate) chart_type: models::ChartType,
    pub(crate) diff_category: models::DifficultyCategory,
    pub(crate) level: String,
    pub(crate) internal_level: Option<f32>,
    pub(crate) achievement_percent: Option<f64>,
    pub(crate) rank: Option<models::ScoreRank>,
    pub(crate) rating_points: Option<u32>,
    pub(crate) image_name: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Bucket {
    New,
    Others,
}

impl Bucket {
    fn label(self) -> &'static str {
        match self {
            Self::New => "NEW",
            Self::Others => "OTHERS",
        }
    }
}

pub(crate) async fn render_rating_image(
    song_info_client: &SongInfoClient,
    new_entries: &[RatingImageEntry],
    old_entries: &[RatingImageEntry],
) -> Result<Vec<u8>> {
    let font = load_font()?;

    let rows = ((new_entries.len() + old_entries.len()) as u32).div_ceil(COLS);
    let width = MARGIN * 2 + CARD_W * COLS + GAP_X * (COLS - 1);
    let height = MARGIN * 2 + HEADER_H + CARD_H * rows + GAP_Y * rows.saturating_sub(1);
    let mut canvas = RgbaImage::from_pixel(width, height, CANVAS_BG);

    draw_header(&mut canvas, &font, new_entries, old_entries);

    let mut cards: Vec<(Bucket, usize, &RatingImageEntry)> = Vec::with_capacity(50);
    cards.extend(
        new_entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| (Bucket::New, idx + 1, entry)),
    );
    cards.extend(
        old_entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| (Bucket::Others, idx + 1, entry)),
    );

    let mut cover_cache: HashMap<String, Option<RgbaImage>> = HashMap::new();

    for (index, (bucket, rank_index, entry)) in cards.iter().enumerate() {
        let col = (index as u32) % COLS;
        let row = (index as u32) / COLS;
        let x = MARGIN + col * (CARD_W + GAP_X);
        let y = MARGIN + HEADER_H + row * (CARD_H + GAP_Y);

        draw_card_shell(&mut canvas, x, y);

        let cover_x = x + 10;
        let cover_y = y + 10;
        let cover = load_cover_image(
            song_info_client,
            entry.image_name.as_ref(),
            &mut cover_cache,
        )
        .await;
        if let Some(cover_img) = cover {
            overlay(&mut canvas, cover_img, cover_x.into(), cover_y.into());
        } else {
            draw_filled_rect_mut(
                &mut canvas,
                Rect::at(cover_x as i32, cover_y as i32).of_size(COVER_SIZE, COVER_SIZE),
                PLACEHOLDER_BG,
            );
            draw_hollow_rect_mut(
                &mut canvas,
                Rect::at(cover_x as i32, cover_y as i32).of_size(COVER_SIZE, COVER_SIZE),
                CARD_BORDER,
            );
            draw_text_mut(
                &mut canvas,
                TEXT_SECONDARY_DARK,
                cover_x as i32 + 15,
                cover_y as i32 + 36,
                15.0,
                &font,
                "NO COVER",
            );
        }

        draw_card_text(&mut canvas, &font, *bucket, *rank_index, entry, x, y);
    }

    let mut png = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .wrap_err("encode rating image as png")?;
    Ok(png)
}

fn draw_header(
    canvas: &mut RgbaImage,
    font: &FontArc,
    new_entries: &[RatingImageEntry],
    old_entries: &[RatingImageEntry],
) {
    let width = canvas.width();
    draw_filled_rect_mut(
        canvas,
        Rect::at(MARGIN as i32, MARGIN as i32).of_size(width - MARGIN * 2, HEADER_H),
        HEADER_BG,
    );

    let new_sum: u32 = new_entries.iter().filter_map(|e| e.rating_points).sum();
    let old_sum: u32 = old_entries.iter().filter_map(|e| e.rating_points).sum();
    let total = new_sum.saturating_add(old_sum);

    draw_text_mut(
        canvas,
        TEXT_LIGHT,
        (MARGIN + 16) as i32,
        (MARGIN + 14) as i32,
        34.0,
        font,
        "maimai Rating Targets",
    );
    draw_text_mut(
        canvas,
        TEXT_LIGHT,
        (MARGIN + 18) as i32,
        (MARGIN + 56) as i32,
        22.0,
        font,
        &format!(
            "NEW {} songs: {} | OTHERS {} songs: {} | TOTAL: {}",
            new_entries.len(),
            new_sum,
            old_entries.len(),
            old_sum,
            total
        ),
    );
}

fn draw_card_shell(canvas: &mut RgbaImage, x: u32, y: u32) {
    draw_filled_rect_mut(
        canvas,
        Rect::at(x as i32, y as i32).of_size(CARD_W, CARD_H),
        CARD_BG,
    );
    draw_hollow_rect_mut(
        canvas,
        Rect::at(x as i32, y as i32).of_size(CARD_W, CARD_H),
        CARD_BORDER,
    );
}

fn draw_card_text(
    canvas: &mut RgbaImage,
    font: &FontArc,
    bucket: Bucket,
    rank_index: usize,
    entry: &RatingImageEntry,
    x: u32,
    y: u32,
) {
    let text_x = x + 114;

    draw_text_mut(
        canvas,
        TEXT_ACCENT,
        text_x as i32,
        (y + 10) as i32,
        17.0,
        font,
        &format!("{} #{rank_index:02}", bucket.label()),
    );

    draw_text_mut(
        canvas,
        TEXT_PRIMARY_DARK,
        text_x as i32,
        (y + 33) as i32,
        22.0,
        font,
        &truncate(&entry.title, 23),
    );

    let level_text = format_level_with_internal(&entry.level, entry.internal_level);
    draw_text_mut(
        canvas,
        TEXT_SECONDARY_DARK,
        text_x as i32,
        (y + 64) as i32,
        18.0,
        font,
        &format!(
            "{} {} {}",
            entry.chart_type, entry.diff_category, level_text
        ),
    );

    let achievement = entry
        .achievement_percent
        .map(|v| format!("{v:.4}%"))
        .unwrap_or_else(|| "N/A".to_string());
    let rank = entry.rank.map(|v| v.as_str()).unwrap_or("N/A");
    draw_text_mut(
        canvas,
        TEXT_SECONDARY_DARK,
        text_x as i32,
        (y + 90) as i32,
        18.0,
        font,
        &format!("Record: {achievement} | {rank}"),
    );

    let rating = entry
        .rating_points
        .map(|v| v.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    draw_text_mut(
        canvas,
        TEXT_PRIMARY_DARK,
        text_x as i32,
        (y + 118) as i32,
        27.0,
        font,
        &format!("Rating: {rating} pt"),
    );
}

async fn load_cover_image<'a>(
    song_info_client: &SongInfoClient,
    image_name: Option<&'a String>,
    cover_cache: &'a mut HashMap<String, Option<RgbaImage>>,
) -> Option<&'a RgbaImage> {
    let image_name = image_name?;

    if !cover_cache.contains_key(image_name) {
        let loaded = match song_info_client.get_cover(image_name).await {
            Ok(bytes) => decode_cover(&bytes),
            Err(e) => {
                tracing::warn!("failed to fetch cover image {image_name}: {e:?}");
                None
            }
        };
        cover_cache.insert(image_name.clone(), loaded);
    }

    cover_cache.get(image_name).and_then(|v| v.as_ref())
}

fn decode_cover(bytes: &[u8]) -> Option<RgbaImage> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    Some(resize(&image, COVER_SIZE, COVER_SIZE, FilterType::Lanczos3))
}

fn truncate(s: &str, max_chars: usize) -> String {
    let mut iter = s.chars();
    let collected: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{collected}...")
    } else {
        collected
    }
}

fn load_font() -> Result<FontArc> {
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var("MAI_RATING_IMG_FONT_PATH") {
        candidates.push(path);
    }

    candidates.extend([
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf".to_string(),
        "/System/Library/Fonts/STHeiti Medium.ttc".to_string(),
        "/System/Library/Fonts/Hiragino Sans GB.ttc".to_string(),
        "/Library/Fonts/Arial Unicode.ttf".to_string(),
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/noto/NotoSansCJK-Regular.ttc".to_string(),
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/TTF/DejaVuSans.ttf".to_string(),
    ]);

    for path in candidates {
        let bytes = match std::fs::read(Path::new(&path)) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Ok(font) = FontArc::try_from_vec(bytes) {
            return Ok(font);
        }
    }

    Err(eyre::eyre!(
        "no usable font found for rating image; set MAI_RATING_IMG_FONT_PATH"
    ))
}
