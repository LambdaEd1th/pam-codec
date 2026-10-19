use pam_codec::{PamInfo, SpriteInfo, decode_pam, encode_pam};
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Real PAM samples are stored in this repository under `sample/` and are also
/// consumed by the Toolkit workspace. `PAM_CODEC_SAMPLE_DIR` can point at any
/// directory with the same layout.
fn sample_dir() -> PathBuf {
    std::env::var_os("PAM_CODEC_SAMPLE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("sample"))
}

fn collect_pams(directory: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_pams(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pam"))
        {
            out.push(path);
        }
    }
}

/// The encoder canonicalizes a zero rotation: it omits the `ROTATE` flag for
/// `[0.0, tx, ty]`, so that transform decodes back as `[tx, ty]`. Normalize both
/// sides before comparing complete documents.
fn canonicalize_zero_rotation(pam: &mut PamInfo) {
    for sprite in pam.sprite.iter_mut().chain(pam.main_sprite.iter_mut()) {
        for frame in &mut sprite.frame {
            for change in &mut frame.change {
                let transform = &mut change.transform;
                if (3..6).contains(&transform.len()) && transform[0].abs() <= f64::EPSILON {
                    transform.remove(0);
                }
            }
        }
    }
}

#[test]
fn sample_pams_decode_encode_decode() {
    let sample_dir = sample_dir();
    let mut samples = Vec::new();
    collect_pams(&sample_dir, &mut samples);
    samples.sort();
    let mut checked = 0;

    for path in samples {
        if !path.is_file() {
            eprintln!("skipping missing PAM sample: {}", path.display());
            continue;
        }

        let mut file = File::open(&path)
            .unwrap_or_else(|error| panic!("{} should open: {error}", path.display()));
        let mut original = decode_pam(&mut file)
            .unwrap_or_else(|error| panic!("{} should decode: {error}", path.display()));

        let mut encoded = Vec::new();
        encode_pam(&original, &mut encoded)
            .unwrap_or_else(|error| panic!("{} should encode: {error}", path.display()));

        let mut cursor = Cursor::new(encoded);
        let mut decoded = decode_pam(&mut cursor)
            .unwrap_or_else(|error| panic!("{} should decode again: {error}", path.display()));

        canonicalize_zero_rotation(&mut original);
        canonicalize_zero_rotation(&mut decoded);
        assert_eq!(decoded, original, "{}", path.display());
        checked += 1;
    }

    if checked == 0 {
        eprintln!(
            "no PAM samples found under {}; set PAM_CODEC_SAMPLE_DIR to enable this test",
            sample_dir.display()
        );
    }
}

#[test]
fn version3_sprites_do_not_gain_frame_rate() {
    let pam = PamInfo {
        version: 3,
        frame_rate: 24,
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        image: Vec::new(),
        sprite: vec![SpriteInfo::default()],
        main_sprite: Some(SpriteInfo::default()),
    };

    let mut encoded = Vec::new();
    encode_pam(&pam, &mut encoded).expect("version 3 PAM should encode");

    let mut cursor = Cursor::new(encoded);
    let decoded = decode_pam(&mut cursor).expect("version 3 PAM should decode");

    assert_eq!(decoded.version, 3);
    assert_eq!(decoded.sprite[0].frame_rate, None);
    assert_eq!(decoded.main_sprite.unwrap().frame_rate, None);
}
