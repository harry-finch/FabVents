//! Archive ZIP minimale (fichiers stockés sans compression, noms UTF-8) :
//! une dalle STEP par fichier. Date fixe (1980-01-01), pour un résultat
//! reproductible.

const DOS_DATE: u16 = 0x0021;

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    !data.iter().fold(!0u32, |c, &b| {
        table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8)
    })
}

fn u16le(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn u32le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Archive des fichiers `(nom, contenu)`.
pub fn zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let name = name.as_bytes();
        // En-tête local.
        u32le(&mut out, 0x0403_4B50);
        for v in [20, 0x0800, 0, 0, DOS_DATE] {
            u16le(&mut out, v);
        }
        for v in [crc, size, size] {
            u32le(&mut out, v);
        }
        u16le(&mut out, name.len() as u16);
        u16le(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        // Entrée du répertoire central.
        u32le(&mut central, 0x0201_4B50);
        for v in [20, 20, 0x0800, 0, 0, DOS_DATE] {
            u16le(&mut central, v);
        }
        for v in [crc, size, size] {
            u32le(&mut central, v);
        }
        for v in [name.len() as u16, 0, 0, 0, 0] {
            u16le(&mut central, v);
        }
        u32le(&mut central, 0);
        u32le(&mut central, offset);
        central.extend_from_slice(name);
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    u32le(&mut out, 0x0605_4B50);
    for v in [0, 0, files.len() as u16, files.len() as u16] {
        u16le(&mut out, v);
    }
    u32le(&mut out, cd_size);
    u32le(&mut out, cd_offset);
    u16le(&mut out, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn archive_layout() {
        let z = zip(&[
            ("a.txt".into(), b"hello".to_vec()),
            ("b.txt".into(), Vec::new()),
        ]);
        assert_eq!(&z[..4], &[0x50, 0x4B, 0x03, 0x04]);
        let end = z.len() - 22;
        assert_eq!(&z[end..end + 4], &[0x50, 0x4B, 0x05, 0x06]);
        assert_eq!(u16::from_le_bytes([z[end + 10], z[end + 11]]), 2);
    }
}
