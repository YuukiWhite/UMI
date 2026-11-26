use regex::Regex;
use std::sync::OnceLock;

/// Evaluate instant local math or currency/unit conversion when the prompt ends with `=`.
/// Examples: `(125*4)/2=`, `15% of 250000=`, `100usd to idr=`, `5km to mi=`, `30c to f=`.
pub fn try_eval(raw_input: &str) -> Option<String> {
    let trimmed = raw_input.trim();
    let expr = trimmed.strip_suffix('=')?.trim();
    if expr.is_empty() {
        return None;
    }

    if let Some(conv) = try_convert(expr) {
        return Some(conv);
    }
    if let Some(pct) = try_percent_of(expr) {
        return Some(pct);
    }
    if let Some(val) = eval_arithmetic(expr) {
        return Some(format_number(val));
    }
    None
}

fn try_percent_of(expr: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^([0-9]+(?:\.[0-9]+)?)\s*%\s*(?:of|dari)\s*([0-9]+(?:\.[0-9]+)?)$")
            .expect("valid pct regex")
    });
    let caps = re.captures(expr)?;
    let p: f64 = caps[1].parse().ok()?;
    let base: f64 = caps[2].parse().ok()?;
    Some(format_number((p / 100.0) * base))
}

fn try_convert(expr: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^([0-9]+(?:[.,][0-9]+)?)\s*([a-z]{1,4})\s+(?:to|in|ke)\s+([a-z]{1,4})$")
            .expect("valid conv regex")
    });
    let caps = re.captures(expr)?;
    let val: f64 = caps[1].replace(',', ".").parse().ok()?;
    let from = caps[2].to_ascii_lowercase();
    let to = caps[3].to_ascii_lowercase();

    // Temperature
    match (from.as_str(), to.as_str()) {
        ("c", "f") => return Some(format!("{:.1} °F", val * 9.0 / 5.0 + 32.0)),
        ("f", "c") => return Some(format!("{:.1} °C", (val - 32.0) * 5.0 / 9.0)),
        ("c", "k") => return Some(format!("{:.2} K", val + 273.15)),
        _ => {}
    }

    // Length (base: meters)
    let len_factor = |u: &str| -> Option<f64> {
        match u {
            "mm" => Some(0.001),
            "cm" => Some(0.01),
            "m" => Some(1.0),
            "km" => Some(1000.0),
            "in" => Some(0.0254),
            "ft" => Some(0.3048),
            "mi" => Some(1609.344),
            _ => None,
        }
    };
    if let (Some(f1), Some(f2)) = (len_factor(&from), len_factor(&to)) {
        let out = val * f1 / f2;
        return Some(format!("{} {}", format_number(out), to));
    }

    // Data size (base: MB)
    let data_factor = |u: &str| -> Option<f64> {
        match u {
            "kb" => Some(1.0 / 1024.0),
            "mb" => Some(1.0),
            "gb" => Some(1024.0),
            "tb" => Some(1024.0 * 1024.0),
            _ => None,
        }
    };
    if let (Some(f1), Some(f2)) = (data_factor(&from), data_factor(&to)) {
        let out = val * f1 / f2;
        return Some(format!("{} {}", format_number(out), to.to_ascii_uppercase()));
    }

    // Approximate offline currency table (base: USD) — instant, zero network
    let usd_rate = |c: &str| -> Option<f64> {
        match c {
            "usd" => Some(1.0),
            "idr" | "rp" => Some(16_200.0),
            "eur" => Some(0.92),
            "gbp" => Some(0.79),
            "jpy" => Some(154.0),
            "sgd" => Some(1.34),
            "myr" => Some(4.42),
            "aud" => Some(1.53),
            _ => None,
        }
    };
    if let (Some(r1), Some(r2)) = (usd_rate(&from), usd_rate(&to)) {
        let out = (val / r1) * r2;
        let label = to.to_ascii_uppercase();
        if label == "IDR" {
            return Some(format!("Rp {} (approx.)", format_thousands(out.round() as i64)));
        }
        return Some(format!("{:.2} {label} (approx.)", out));
    }

    None
}

fn format_thousands(n: i64) -> String {
    let neg = n < 0;
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (idx, ch) in s.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if neg {
        out.push('-');
    }
    out.chars().rev().collect()
}

fn format_number(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e15 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Safe recursive-descent arithmetic evaluator (+, -, *, /, ^, parentheses).
fn eval_arithmetic(input: &str) -> Option<f64> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let res = parse_expr(&tokens, &mut pos)?;
    if pos == tokens.len() && res.is_finite() {
        Some(res)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    Num(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' => i += 1,
            b'+' => { out.push(Tok::Plus); i += 1; }
            b'-' => { out.push(Tok::Minus); i += 1; }
            b'*' | b'x' | b'X' => { out.push(Tok::Star); i += 1; }
            b'/' => { out.push(Tok::Slash); i += 1; }
            b'^' => { out.push(Tok::Caret); i += 1; }
            b'(' => { out.push(Tok::LParen); i += 1; }
            b')' => { out.push(Tok::RParen); i += 1; }
            b'0'..=b'9' | b'.' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                let val: f64 = s[start..i].parse().ok()?;
                out.push(Tok::Num(val));
            }
            _ => return None,
        }
    }
    Some(out)
}

fn parse_expr(t: &[Tok], p: &mut usize) -> Option<f64> {
    let mut left = parse_term(t, p)?;
    while *p < t.len() {
        match t[*p] {
            Tok::Plus => { *p += 1; left += parse_term(t, p)?; }
            Tok::Minus => { *p += 1; left -= parse_term(t, p)?; }
            _ => break,
        }
    }
    Some(left)
}

fn parse_term(t: &[Tok], p: &mut usize) -> Option<f64> {
    let mut left = parse_power(t, p)?;
    while *p < t.len() {
        match t[*p] {
            Tok::Star => { *p += 1; left *= parse_power(t, p)?; }
            Tok::Slash => {
                *p += 1;
                let rhs = parse_power(t, p)?;
                if rhs.abs() < 1e-12 {
                    return None;
                }
                left /= rhs;
            }
            _ => break,
        }
    }
    Some(left)
}

fn parse_power(t: &[Tok], p: &mut usize) -> Option<f64> {
    let base = parse_unary(t, p)?;
    if *p < t.len() && t[*p] == Tok::Caret {
        *p += 1;
        let exp = parse_power(t, p)?;
        return Some(base.powf(exp));
    }
    Some(base)
}

fn parse_unary(t: &[Tok], p: &mut usize) -> Option<f64> {
    if *p < t.len() {
        match t[*p] {
            Tok::Minus => { *p += 1; return Some(-parse_unary(t, p)?); }
            Tok::Plus => { *p += 1; return parse_unary(t, p); }
            _ => {}
        }
    }
    parse_primary(t, p)
}

fn parse_primary(t: &[Tok], p: &mut usize) -> Option<f64> {
    if *p >= t.len() {
        return None;
    }
    match t[*p] {
        Tok::Num(v) => { *p += 1; Some(v) }
        Tok::LParen => {
            *p += 1;
            let inner = parse_expr(t, p)?;
            if *p < t.len() && t[*p] == Tok::RParen {
                *p += 1;
                Some(inner)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_math_and_currency_locally() {
        assert_eq!(try_eval("(125*4)/2="), Some("250".to_string()));
        assert_eq!(try_eval("2^8 + 4="), Some("260".to_string()));
        assert_eq!(try_eval("15% of 200000="), Some("30000".to_string()));
        assert_eq!(try_eval("100usd to idr="), Some("Rp 1.620.000 (approx.)".to_string()));
        assert_eq!(try_eval("30c to f="), Some("86.0 °F".to_string()));
        assert_eq!(try_eval("not math"), None);
    }
}
