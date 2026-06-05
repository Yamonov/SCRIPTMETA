use std::borrow::Cow;

use encoding_rs::{EUC_JP, SHIFT_JIS, UTF_16BE, UTF_16LE};

#[must_use]
pub fn decode_script_text(bytes: &[u8]) -> String {
    decode_script_text_strict(bytes)
        .map(Cow::into_owned)
        .unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
}

#[must_use]
pub fn decode_script_text_strict(bytes: &[u8]) -> Option<Cow<'_, str>> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF])
        && let Ok(text) = std::str::from_utf8(&bytes[3..])
    {
        return Some(Cow::Borrowed(text));
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_with_encoding(UTF_16LE, &bytes[2..]);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_with_encoding(UTF_16BE, &bytes[2..]);
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return Some(Cow::Borrowed(text));
    }

    decode_with_encoding(EUC_JP, bytes)
        .or_else(|| decode_with_encoding(SHIFT_JIS, bytes))
        .or_else(|| decode_with_encoding(UTF_16LE, bytes))
        .or_else(|| decode_with_encoding(UTF_16BE, bytes))
}

fn decode_with_encoding<'a>(
    encoding: &'static encoding_rs::Encoding,
    bytes: &'a [u8],
) -> Option<Cow<'a, str>> {
    let (text, _encoding_used, had_errors) = encoding.decode(bytes);
    (!had_errors).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::decode_script_text;

    #[test]
    fn decodes_shift_jis_script_text() {
        let bytes = [
            0x2f, 0x2f, 0x20, 0x83, 0x58, 0x83, 0x4e, 0x83, 0x8a, 0x83, 0x76, 0x83, 0x67,
        ];

        assert_eq!(decode_script_text(&bytes), "// スクリプト");
    }

    #[test]
    fn decodes_utf16le_with_bom() {
        let bytes = [
            0xff, 0xfe, 0x53, 0x00, 0x43, 0x00, 0x52, 0x00, 0x49, 0x00, 0x50, 0x00,
        ];

        assert_eq!(decode_script_text(&bytes), "SCRIP");
    }
}
