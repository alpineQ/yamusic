pub fn parse_lrc(text: &str) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    let mut seq = 0usize;

    for line in text.lines() {
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut timestamps = Vec::new();
        let mut last_end = 0;

        while i < bytes.len() {
            if bytes[i] == b'[' {
                let start = i + 1;
                if let Some(end) = line[start..].find(']').map(|e| start + e) {
                    let tag = &line[start..end];
                    if let Some(ts) = parse_timestamp(tag) {
                        timestamps.push(ts);
                        last_end = end + 1;
                    }
                    i = end + 1;
                    continue;
                } else {
                    break;
                }
            }
            i += 1;
        }

        if timestamps.is_empty() {
            continue;
        }

        let content_slice = line[last_end..].trim();
        let content = if content_slice.is_empty() {
            String::new()
        } else {
            content_slice.to_string()
        };

        for t in timestamps {
            out.push((t, seq, content.clone()));
            seq += 1;
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out.into_iter().map(|(t, _, line)| (t, line)).collect()
}

#[inline]
fn parse_timestamp(tag: &str) -> Option<u64> {
    let mut parts = tag.split(':');
    let min = parts.next()?.parse::<u64>().ok()?;
    let rest = parts.next()?;

    let (sec, ms) = if let Some(dot) = rest.find('.') {
        let sec = rest[..dot].parse::<u64>().ok()?;
        if sec > 59 {
            return None;
        }

        let mut frac_value = 0u64;
        let mut digits = 0u32;
        for b in rest[dot + 1..].bytes() {
            if !b.is_ascii_digit() {
                break;
            }
            if digits == 3 {
                break;
            }
            frac_value = frac_value * 10 + (b - b'0') as u64;
            digits += 1;
        }

        if digits == 0 {
            return None;
        }

        let ms = match digits {
            1 => frac_value * 100,
            2 => frac_value * 10,
            _ => frac_value,
        };
        (sec, ms)
    } else {
        let sec = rest.parse().ok()?;
        if sec > 59 {
            return None;
        }
        (sec, 0)
    };

    Some((min * 60 + sec) * 1000 + ms)
}
