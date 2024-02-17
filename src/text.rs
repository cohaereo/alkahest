/// Expects raw un-shifted data as input
/// Currently very incomplete
// TODO(cohae): Support for wide characters
pub fn decode_text(data: &[u8], cipher: u16) -> String {
    if cipher == 0 {
        return String::from_utf8_lossy(data).to_string();
    }

    let mut result = String::new();

    let mut offset = 0;
    while offset < data.len() {
        let b0 = data[offset];
        let u0 = b0.wrapping_add(cipher as u8);

        match b0 {
            0xc0..=0xdf => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 2
            }
            0xe0..=0xef => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 3
            }
            0..=0x7f => {
                result.push(char::from(u0));
                offset += 1
            }
            _ => {
                result.push(char::REPLACEMENT_CHARACTER);
                offset += 1
            }
        }
    }

    result
}
