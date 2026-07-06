use crate::types::{CivilDateTime, DetectionError};

pub(super) fn parse_natural_deadline(text: &str) -> Result<Option<CivilDateTime>, ()> {
    let words = sanitized_words(text);
    for window in words.windows(3) {
        let [month, day, year] = window else {
            continue;
        };
        let Some(month) = month_number(month) else {
            continue;
        };
        let Some(day) = parse_day(day) else {
            continue;
        };
        let Some(year) = parse_year(year) else {
            continue;
        };
        return parse_deadline_date(year, month, day).map(Some);
    }
    Ok(None)
}

pub(super) fn parse_explicit_datetime(text: &str) -> Result<Option<CivilDateTime>, ()> {
    let words = sanitized_words(text);
    let date = words.iter().find(|word| looks_like_date(word));
    let time = words
        .iter()
        .enumerate()
        .find(|(_, word)| looks_like_time(word));
    match (date, time) {
        (Some(date), Some((time_index, time))) => {
            let meridiem = words
                .get(time_index + 1)
                .and_then(|word| Meridiem::parse(word));
            parse_date_and_time(date, time, meridiem).map(Some)
        }
        (Some(_), None) | (None, Some(_)) | (None, None) => Ok(None),
    }
}

fn parse_deadline_date(year: u16, month: u8, day: u8) -> Result<CivilDateTime, ()> {
    let normalized = format!("{year:04}-{month:02}-{day:02}T23:59:00");
    CivilDateTime::parse_reference(&normalized).map_err(|err| match err {
        DetectionError::InvalidInput {
            field: _,
            reason: _,
        } => (),
    })
}

fn month_number(word: &str) -> Option<u8> {
    if word.eq_ignore_ascii_case("january") || word.eq_ignore_ascii_case("jan") {
        Some(1)
    } else if word.eq_ignore_ascii_case("february") || word.eq_ignore_ascii_case("feb") {
        Some(2)
    } else if word.eq_ignore_ascii_case("march") || word.eq_ignore_ascii_case("mar") {
        Some(3)
    } else if word.eq_ignore_ascii_case("april") || word.eq_ignore_ascii_case("apr") {
        Some(4)
    } else if word.eq_ignore_ascii_case("may") {
        Some(5)
    } else if word.eq_ignore_ascii_case("june") || word.eq_ignore_ascii_case("jun") {
        Some(6)
    } else if word.eq_ignore_ascii_case("july") || word.eq_ignore_ascii_case("jul") {
        Some(7)
    } else if word.eq_ignore_ascii_case("august") || word.eq_ignore_ascii_case("aug") {
        Some(8)
    } else if word.eq_ignore_ascii_case("september") || word.eq_ignore_ascii_case("sep") {
        Some(9)
    } else if word.eq_ignore_ascii_case("october") || word.eq_ignore_ascii_case("oct") {
        Some(10)
    } else if word.eq_ignore_ascii_case("november") || word.eq_ignore_ascii_case("nov") {
        Some(11)
    } else if word.eq_ignore_ascii_case("december") || word.eq_ignore_ascii_case("dec") {
        Some(12)
    } else {
        None
    }
}

fn parse_day(word: &str) -> Option<u8> {
    word.parse::<u8>().ok()
}

fn parse_year(word: &str) -> Option<u16> {
    if word.len() == 4 {
        word.parse::<u16>().ok()
    } else {
        None
    }
}

#[derive(Clone, Copy)]
enum Meridiem {
    Am,
    Pm,
}

impl Meridiem {
    fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "am" | "a.m" => Some(Self::Am),
            "pm" | "p.m" => Some(Self::Pm),
            _ => None,
        }
    }
}

fn parse_date_and_time(
    date: &str,
    time: &str,
    meridiem: Option<Meridiem>,
) -> Result<CivilDateTime, ()> {
    let (hour, minute) = parse_clock_time(time, meridiem)?;
    let normalized = format!("{date}T{hour:02}:{minute:02}:00");
    CivilDateTime::parse_reference(&normalized).map_err(|err| match err {
        DetectionError::InvalidInput {
            field: _,
            reason: _,
        } => (),
    })
}

fn parse_clock_time(time: &str, meridiem: Option<Meridiem>) -> Result<(u8, u8), ()> {
    let (hour_text, minute_text) = time.split_once(':').ok_or(())?;
    if minute_text.len() != 2 || minute_text.contains(':') {
        return Err(());
    }
    let hour = hour_text.parse::<u8>().map_err(|_| ())?;
    let minute = minute_text.parse::<u8>().map_err(|_| ())?;
    if minute > 59 {
        return Err(());
    }
    let hour = match meridiem {
        None if hour <= 23 => hour,
        Some(Meridiem::Am) if (1..=12).contains(&hour) => {
            if hour == 12 {
                0
            } else {
                hour
            }
        }
        Some(Meridiem::Pm) if (1..=12).contains(&hour) => {
            if hour == 12 {
                12
            } else {
                hour + 12
            }
        }
        None | Some(_) => return Err(()),
    };
    Ok((hour, minute))
}

fn sanitized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(trim_punctuation).to_owned())
        .collect()
}

fn trim_punctuation(ch: char) -> bool {
    matches!(
        ch,
        ',' | '.' | '?' | '!' | ';' | ':' | '(' | ')' | '[' | ']'
    )
}

fn looks_like_date(word: &str) -> bool {
    let mut parts = word.split('-');
    let year = parts.next().is_some_and(|part| part.len() == 4);
    let month = parts.next().is_some_and(|part| part.len() == 2);
    let day = parts.next().is_some_and(|part| part.len() == 2);
    year && month && day && parts.next().is_none()
}

fn looks_like_time(word: &str) -> bool {
    let mut parts = word.split(':');
    let hour = parts.next().is_some_and(|part| !part.is_empty());
    let minute = parts.next().is_some_and(|part| part.len() == 2);
    hour && minute
}
