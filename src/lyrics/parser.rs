use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]").expect("valid lrc regex"))
}

/// Parse standard LRC text (`[mm:ss.xx] Line`) into a sorted list of timed lines.
pub fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let re = tag_re();
    let mut lines = Vec::new();

    for raw_line in lrc.lines() {
        let mut timestamps = Vec::new();
        let mut last_end = 0;

        for caps in re.captures_iter(raw_line) {
            let mins: u64 = caps[1].parse().unwrap_or(0);
            let secs: u64 = caps[2].parse().unwrap_or(0);
            let frac_ms: u64 = caps
                .get(3)
                .map(|m| {
                    let s = m.as_str();
                    let val: u64 = s.parse().unwrap_or(0);
                    match s.len() {
                        1 => val * 100,
                        2 => val * 10,
                        _ => val,
                    }
                })
                .unwrap_or(0);
            timestamps.push(mins * 60_000 + secs * 1_000 + frac_ms);
            if let Some(m) = caps.get(0) {
                last_end = m.end();
            }
        }

        if timestamps.is_empty() {
            continue;
        }

        let text = raw_line[last_end..].trim().to_string();
        if text.is_empty() {
            continue;
        }
        for ts in timestamps {
            lines.push(LyricLine {
                time_ms: ts,
                text: text.clone(),
            });
        }
    }

    lines.sort_by_key(|l| l.time_ms);
    lines
}

/// Binary search the active lyric line index for the given playback position (ms).
pub fn active_line_index(lines: &[LyricLine], position_ms: u64) -> Option<usize> {
    if lines.is_empty() {
        return None;
    }
    let idx = lines.partition_point(|l| l.time_ms <= position_ms);
    if idx == 0 {
        Some(0)
    } else {
        Some(idx - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_and_finds_active_line() {
        let sample = "\
[ar:Summer Coast]
[00:05.50]drifting through the quiet night
[00:12.00][01:02.25]city lights fade out of sight
[00:18.75]till the morning comes around
";
        let lines = parse_lrc(sample);
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0].time_ms, 5500);
        assert_eq!(lines[1].time_ms, 12000);
        assert_eq!(lines[2].time_ms, 18750);
        assert_eq!(lines[3].time_ms, 62250);

        assert_eq!(active_line_index(&lines, 4000), Some(0));
        assert_eq!(active_line_index(&lines, 13000), Some(1));
        assert_eq!(active_line_index(&lines, 20000), Some(2));
        assert_eq!(active_line_index(&lines, 65000), Some(3));
    }
}
