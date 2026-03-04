use maimai_parsers::parse_playlog_detail_html;

#[test]
fn parse_playlog_detail_extracts_title_and_music_detail_idx() {
    let html = r#"
    <html>
      <body>
        <div class="basic_block">
          <div class="f_15 break">Link</div>
        </div>
        <form action="https://maimaidx-eng.com/maimai-mobile/record/musicDetail/" method="get">
          <input type="hidden" name="idx" value="music-detail-idx-123" />
          <button type="submit">MY RECORD</button>
        </form>
      </body>
    </html>
    "#;

    let parsed = parse_playlog_detail_html(html).unwrap();
    assert_eq!(parsed.title, "Link");
    assert_eq!(parsed.music_detail_idx, "music-detail-idx-123");
}
