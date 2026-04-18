use rand::RngCore;

use crate::config::PayloadPaddingConfig;

const TPL_HEX: &str = "1603010200010001fc030341d5b549d9cd1adfa7296c8418d157dc7b624c842824ff493b9375bb48d34f2b20bf018bcc90a7c89a230094815ad0c15b736e38c01209d72d282cb5e2105328150024130213031301c02cc030c02bc02fcca9cca8c024c028c023c027009f009e006b006700ff0100018f0000000b00090000066d63692e6972000b000403000102000a00160014001d0017001e0019001801000101010201030104002300000010000e000c02683208687474702f312e310016000000170000000d002a0028040305030603080708080809080a080b080408050806040105010601030303010302040205020602002b00050403040303002d00020101003300260024001d0020435bacc4d05f9d41fef44ab3ad55616c36e0613473e2338770efdaa98693d217001500d5";

const TEMPLATE_SNI: &[u8] = b"mci.ir";

pub const CLIENT_HELLO_SIZE: usize = 517;

fn template_bytes() -> Vec<u8> {
    hex::decode(TPL_HEX).expect("invalid template hex")
}

mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("odd length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string())
            })
            .collect()
    }
}

/// Build a fake TLS ClientHello for the given SNI.
/// `extra_padding` bytes are added to the 0x0015 padding extension, varying the
/// overall packet size to defeat fixed-length DPI fingerprinting.
pub fn build_client_hello(sni: &str, extra_padding: usize) -> Vec<u8> {
    assert!(sni.len() <= 219, "SNI too long (max 219 bytes)");

    let tpl = template_bytes();
    let sni_bytes = sni.as_bytes();
    let tpl_sni_len = TEMPLATE_SNI.len();

    let static1 = &tpl[..11];
    let static3 = &tpl[76..120];
    let static4 = &tpl[127 + tpl_sni_len..262 + tpl_sni_len];

    let mut rng = rand::thread_rng();
    let mut random = [0u8; 32];
    let mut sess_id = [0u8; 32];
    let mut key_share = [0u8; 32];
    rng.fill_bytes(&mut random);
    rng.fill_bytes(&mut sess_id);
    rng.fill_bytes(&mut key_share);

    let pad_len = 219 - sni_bytes.len();

    let mut out = Vec::with_capacity(CLIENT_HELLO_SIZE + extra_padding);

    out.extend_from_slice(static1);
    out.extend_from_slice(&random);
    out.push(0x20);
    out.extend_from_slice(&sess_id);
    out.extend_from_slice(static3);

    let sni_ext_len = (sni_bytes.len() + 5) as u16;
    let sni_list_len = (sni_bytes.len() + 3) as u16;
    let sni_len = sni_bytes.len() as u16;
    out.extend_from_slice(&sni_ext_len.to_be_bytes());
    out.extend_from_slice(&sni_list_len.to_be_bytes());
    out.push(0x00);
    out.extend_from_slice(&sni_len.to_be_bytes());
    out.extend_from_slice(sni_bytes);

    out.extend_from_slice(static4);
    out.extend_from_slice(&key_share);

    out.extend_from_slice(&[0x00, 0x15]);
    out.extend_from_slice(&(pad_len as u16).to_be_bytes());
    out.extend_from_slice(&vec![0x00; pad_len]);

    assert_eq!(out.len(), CLIENT_HELLO_SIZE, "ClientHello size mismatch: got {}", out.len());

    if extra_padding > 0 {
        // Patch TLS record length at bytes [3..5] (u16 big-endian)
        let tls_record_len = u16::from_be_bytes([out[3], out[4]]) + extra_padding as u16;
        out[3..5].copy_from_slice(&tls_record_len.to_be_bytes());

        // Patch Handshake length at bytes [6..9] (3-byte big-endian)
        let hs_len = ((out[6] as u32) << 16) | ((out[7] as u32) << 8) | (out[8] as u32);
        let hs_len = hs_len + extra_padding as u32;
        out[6] = (hs_len >> 16) as u8;
        out[7] = (hs_len >> 8) as u8;
        out[8] = hs_len as u8;

        // Patch extensions total-length at bytes [116..118] (u16 big-endian)
        // (position in out is fixed: static3 starts at 76, ext total len is at offset 40 within static3)
        let ext_total_len = u16::from_be_bytes([out[116], out[117]]) + extra_padding as u16;
        out[116..118].copy_from_slice(&ext_total_len.to_be_bytes());

        // Patch the padding extension's own length field
        // padding ext starts at: 294 + sni.len() → [0x00,0x15, len_hi, len_lo, zeros...]
        //                                             ^294       ^296
        let pad_ext_len_pos = 296 + sni_bytes.len();
        let cur_pad_ext_len = u16::from_be_bytes([out[pad_ext_len_pos], out[pad_ext_len_pos + 1]]);
        let new_pad_ext_len = cur_pad_ext_len + extra_padding as u16;
        out[pad_ext_len_pos..pad_ext_len_pos + 2].copy_from_slice(&new_pad_ext_len.to_be_bytes());

        // Append extra padding bytes
        out.extend_from_slice(&vec![0x00; extra_padding]);
    }

    assert_eq!(out.len(), CLIENT_HELLO_SIZE + extra_padding, "ClientHello size mismatch: got {}", out.len());
    out
}

/// Build a fake ClientHello with random extra padding drawn from `padding_cfg`.
/// When padding is disabled (max_extra_bytes == 0), behaves identically to
/// `build_client_hello(sni, 0)` and always produces CLIENT_HELLO_SIZE bytes.
pub fn build_client_hello_padded(sni: &str, padding_cfg: &PayloadPaddingConfig) -> Vec<u8> {
    let extra = if padding_cfg.is_disabled() {
        0
    } else {
        use rand::Rng;
        rand::thread_rng().gen_range(padding_cfg.min_extra_bytes..=padding_cfg.max_extra_bytes)
    };
    build_client_hello(sni, extra)
}

pub fn parse_sni(client_hello: &[u8]) -> Option<String> {
    // Only needs bytes up to 129; SNI offset is fixed regardless of total packet length.
    if client_hello.len() < 129 {
        return None;
    }
    let sni_len = u16::from_be_bytes([client_hello[125], client_hello[126]]) as usize;
    if 127 + sni_len > client_hello.len() {
        return None;
    }
    String::from_utf8(client_hello[127..127 + sni_len].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_hello_size() {
        let ch = build_client_hello("example.com", 0);
        assert_eq!(ch.len(), CLIENT_HELLO_SIZE);
    }

    #[test]
    fn test_client_hello_sni() {
        let ch = build_client_hello("security.vercel.com", 0);
        let sni = parse_sni(&ch).unwrap();
        assert_eq!(sni, "security.vercel.com");
    }

    #[test]
    fn test_client_hello_short_sni() {
        let ch = build_client_hello("a.b", 0);
        assert_eq!(ch.len(), CLIENT_HELLO_SIZE);
        let sni = parse_sni(&ch).unwrap();
        assert_eq!(sni, "a.b");
    }

    #[test]
    fn test_client_hello_max_sni() {
        let sni = "a".repeat(219);
        let ch = build_client_hello(&sni, 0);
        assert_eq!(ch.len(), CLIENT_HELLO_SIZE);
        let parsed = parse_sni(&ch).unwrap();
        assert_eq!(parsed, sni);
    }

    #[test]
    fn test_tls_record_header() {
        let ch = build_client_hello("test.com", 0);
        assert_eq!(ch[0], 0x16);
        assert_eq!(ch[1], 0x03);
        assert_eq!(ch[2], 0x01);
    }

    #[test]
    fn test_client_hello_padding() {
        let ch = build_client_hello("example.com", 64);
        assert_eq!(ch.len(), CLIENT_HELLO_SIZE + 64);
        // TLS record length should be updated
        let tls_len = u16::from_be_bytes([ch[3], ch[4]]) as usize;
        assert_eq!(tls_len, CLIENT_HELLO_SIZE - 5 + 64);
        // SNI should still be parseable
        let sni = parse_sni(&ch).unwrap();
        assert_eq!(sni, "example.com");
    }

    #[test]
    fn test_client_hello_padded_disabled() {
        let cfg = PayloadPaddingConfig::default();
        let ch = build_client_hello_padded("example.com", &cfg);
        assert_eq!(ch.len(), CLIENT_HELLO_SIZE);
    }
}
