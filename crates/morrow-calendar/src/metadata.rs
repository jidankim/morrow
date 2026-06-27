use crate::{CalendarError, CalendarSourceId, CandidateId, ProposalMetadata, VideoUrl};

const BEGIN: &str = "[MORROW_METADATA_V1]";
const END: &str = "[/MORROW_METADATA_V1]";

pub fn format_notes(
    user_note: &str,
    video_url: Option<&VideoUrl>,
    metadata: &ProposalMetadata,
) -> String {
    let mut notes = String::with_capacity(user_note.len() + 160);
    notes.push_str(user_note);
    if !user_note.is_empty() {
        notes.push('\n');
    }
    notes.push_str(BEGIN);
    notes.push('\n');
    notes.push_str("candidate_id=");
    notes.push_str(&hex_encode(metadata.candidate_id.as_str()));
    notes.push('\n');
    notes.push_str("source_id=");
    notes.push_str(&hex_encode(metadata.source_id.as_str()));
    notes.push('\n');
    if let Some(url) = video_url {
        notes.push_str("video_url=");
        notes.push_str(url.as_str());
        notes.push('\n');
    }
    notes.push_str(END);
    notes
}

pub fn parse_metadata(notes: &str) -> Result<ProposalMetadata, CalendarError> {
    let body = metadata_body(notes)?;
    let mut candidate_id = None;
    let mut source_id = None;

    for line in body.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "candidate_id" => candidate_id = Some(CandidateId::new(&hex_decode(value)?)?),
                "source_id" => source_id = Some(CalendarSourceId::new(&hex_decode(value)?)?),
                "video_url" => {
                    let _url = VideoUrl::new(value)?;
                }
                _ => {}
            }
        }
    }

    let candidate_id = candidate_id.ok_or_else(|| CalendarError::MetadataInvalid {
        reason: "candidate_id is missing".to_owned(),
    })?;
    let source_id = source_id.ok_or_else(|| CalendarError::MetadataInvalid {
        reason: "source_id is missing".to_owned(),
    })?;
    Ok(ProposalMetadata {
        candidate_id,
        source_id,
    })
}

pub fn strip_morrow_metadata(notes: &str) -> &str {
    let Some(end_index) = notes.rfind(END) else {
        return notes;
    };
    let Some(begin_index) = notes[..end_index].rfind(BEGIN) else {
        return notes;
    };
    if begin_index > 0 && notes.as_bytes().get(begin_index - 1) == Some(&b'\n') {
        &notes[..begin_index - 1]
    } else {
        &notes[..begin_index]
    }
}

fn metadata_body(notes: &str) -> Result<&str, CalendarError> {
    let end_index = notes.rfind(END).ok_or(CalendarError::MetadataMissing)?;
    let begin_index = notes[..end_index]
        .rfind(BEGIN)
        .ok_or(CalendarError::MetadataMissing)?;
    Ok(&notes[begin_index + BEGIN.len()..end_index])
}

fn hex_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn hex_decode(input: &str) -> Result<String, CalendarError> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(CalendarError::MetadataInvalid {
            reason: "hex value has odd length".to_owned(),
        });
    }

    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        decoded.push((high << 4) | low);
    }

    String::from_utf8(decoded).map_err(|_| CalendarError::MetadataInvalid {
        reason: "hex value is not utf-8".to_owned(),
    })
}

fn hex_digit(byte: u8) -> Result<u8, CalendarError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(CalendarError::MetadataInvalid {
            reason: "hex value contains a non-hex digit".to_owned(),
        }),
    }
}
