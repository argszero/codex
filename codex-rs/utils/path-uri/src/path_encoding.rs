//! Percent-encoding of native path segments when building `file:` URLs.

use url::Url;

/// Percent-encodes one native path segment for a `file:` URL path.
///
/// The URL path setter cannot be used for this directly: the WHATWG path state
/// strips ASCII tab (`0x09`), LF (`0x0A`) and CR (`0x0D`) from its input, so a
/// segment containing one of them would silently name a different path. Those
/// three are the only characters the setter drops, so every other run of the
/// segment is encoded by the setter itself and joined with explicit escapes. A
/// segment that contains none of them is therefore encoded exactly as the setter
/// would have encoded it.
pub(super) fn encode_path_segment(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    let mut run = String::new();
    for character in segment.chars() {
        let escape = match character {
            '\t' => "%09",
            '\n' => "%0A",
            '\r' => "%0D",
            character => {
                run.push(character);
                continue;
            }
        };
        encoded.push_str(&encode_run(&mut run));
        encoded.push_str(escape);
    }
    encoded.push_str(&encode_run(&mut run));
    encoded
}

/// Percent-encodes one run of characters that the URL path setter preserves.
///
/// The run is handed to the setter rather than encoded from a local table so the
/// resulting spelling stays identical to what a direct path-setter push produced
/// before the characters the setter drops had to be separated out.
fn encode_run(run: &mut String) -> String {
    if run.is_empty() {
        return String::new();
    }
    let Ok(mut url) = Url::parse("file:///") else {
        unreachable!("the file URL prefix is valid");
    };
    {
        let Ok(mut segments) = url.path_segments_mut() else {
            unreachable!("file URLs support hierarchical path segments");
        };
        segments.push(run);
    }
    let encoded = url.path().trim_start_matches('/').to_string();
    run.clear();
    encoded
}
