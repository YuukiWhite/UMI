use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanTrack {
    pub artist: String,
    pub title: String,
    pub cache_key: String,
}

fn brackets_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)[\(\[\{]\s*(official\s*(music\s*)?(video|audio|mv|lyric(s)?(\s*video)?)|lyrics?|audio|mv|hd|4k|hq|visualizer|live(\s+at[^)\]\}]*)?|remaster(ed)?(\s+\d{4})?|full\s+song|prod\.?[^)\]\}]*|feat\.?[^)\]\}]*|ft\.?[^)\]\}]*)\s*[\)\]\}]",
        )
        .expect("valid regex")
    })
}

fn trailing_pipe_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s*[\|｜]\s*.*$").expect("valid regex"))
}

fn feat_inline_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\s+(feat\.?|ft\.?|featuring)\s+.*$").expect("valid regex"))
}

fn topic_vevo_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)(\s*-\s*topic|vevo|\s+official)$").expect("valid regex"))
}

fn multi_space_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s{2,}").expect("valid regex"))
}

/// Normalize raw MPRIS2 metadata from Spotify, YouTube, and YouTube Music before LRCLIB lookup.
pub fn sanitize_track(raw_artist: &str, raw_title: &str) -> CleanTrack {
    let mut artist = topic_vevo_re().replace(raw_artist.trim(), "").trim().to_string();
    let mut title = raw_title.trim().to_string();

    // Strip YouTube / YouTube Music noisy suffixes like "| Official Video"
    title = trailing_pipe_re().replace(&title, "").to_string();
    // Repeatedly strip bracketed video/audio/lyric tags
    loop {
        let next = brackets_re().replace_all(&title, "").to_string();
        if next == title {
            break;
        }
        title = next;
    }

    // If artist is empty or looks like a generic YouTube channel, split "Artist - Song Title"
    let artist_generic = artist.is_empty()
        || artist.eq_ignore_ascii_case("youtube")
        || artist.eq_ignore_ascii_case("youtube music")
        || artist.eq_ignore_ascii_case("various artists")
        || artist.to_ascii_lowercase().contains("lyrics")
        || artist.to_ascii_lowercase().contains("music");

    if let Some((lhs, rhs)) = split_artist_title(&title) {
        if artist_generic || lhs.eq_ignore_ascii_case(&artist) {
            artist = topic_vevo_re().replace(lhs.trim(), "").trim().to_string();
            title = rhs.trim().to_string();
        }
    }

    title = feat_inline_re().replace(&title, "").to_string();
    title = title.trim_matches(|c: char| c == '-' || c == '"' || c == '—' || c == '–').trim().to_string();
    title = multi_space_re().replace_all(&title, " ").trim().to_string();
    artist = multi_space_re().replace_all(&artist, " ").trim().to_string();

    let cache_key = format!(
        "{}::{}",
        artist.to_ascii_lowercase(),
        title.to_ascii_lowercase()
    );

    CleanTrack {
        artist,
        title,
        cache_key,
    }
}

fn split_artist_title(s: &str) -> Option<(&str, &str)> {
    for sep in [" - ", " – ", " — "] {
        if let Some((l, r)) = s.split_once(sep) {
            if !l.trim().is_empty() && !r.trim().is_empty() {
                return Some((l, r));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_youtube_and_ytm_titles() {
        let yt = sanitize_track(
            "YouTube",
            "NIKI - Lowkey (Official Music Video) [HD] | 88rising",
        );
        assert_eq!(yt.artist, "NIKI");
        assert_eq!(yt.title, "Lowkey");

        let ytm = sanitize_track("Tulus - Topic", "Hati-Hati di Jalan (Official Audio)");
        assert_eq!(ytm.artist, "Tulus");
        assert_eq!(ytm.title, "Hati-Hati di Jalan");

        let spot = sanitize_track("The Weeknd", "Blinding Lights (Remastered 2020)");
        assert_eq!(spot.artist, "The Weeknd");
        assert_eq!(spot.title, "Blinding Lights");

        let feat = sanitize_track("Daft Punk", "Get Lucky ft. Pharrell Williams (Official Video)");
        assert_eq!(feat.artist, "Daft Punk");
        assert_eq!(feat.title, "Get Lucky");
    }
}
