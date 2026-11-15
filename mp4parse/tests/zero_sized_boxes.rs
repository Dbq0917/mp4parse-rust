//! Tests for top-level boxes with a size field of 0, which extend to the
//! end of the file per ISOBMFF (ISO 14496-12:2020) § 4.2. Such boxes are
//! common in files produced by recorders (e.g. surveillance cameras) which
//! cannot seek back to finalize the box size, and may carry proprietary
//! extension box types in addition to the traditional `mdat`.
use std::convert::TryInto;

use mp4parse as mp4;
use std::io::Cursor;

static MINI_MP4: &str = "tests/minimal.mp4";

/// Split a file into its top-level boxes.
fn split_boxes(data: &[u8]) -> Vec<Vec<u8>> {
    let mut boxes = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let size = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        boxes.push(data[offset..offset + size].to_vec());
        offset += size;
    }
    boxes
}

/// Append a box with a size field of 0 ("extends to end of file").
fn append_zero_sized_box(out: &mut Vec<u8>, name: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(payload);
}

fn read_mp4(data: &[u8]) -> Result<mp4::MediaContext, mp4::Error> {
    let mut stream = Cursor::new(data);
    mp4::read_mp4(&mut stream, mp4::ParseStrictness::Normal)
}

#[test]
fn zero_sized_mdat_at_end() {
    let data = std::fs::read(MINI_MP4).expect("read minimal.mp4");
    let boxes = split_boxes(&data);
    let [ftyp, moov, free, mdat] = &boxes[..] else {
        panic!("unexpected box layout in minimal.mp4")
    };
    assert_eq!(&mdat[4..8], b"mdat");

    let mut file = Vec::new();
    for b in [ftyp, moov, free] {
        file.extend_from_slice(b);
    }
    append_zero_sized_box(&mut file, b"mdat", &mdat[8..]);

    let context = read_mp4(&file).expect("zero-sized mdat should parse");
    assert_eq!(context.tracks.len(), 2);
}

#[test]
fn zero_sized_unknown_box_at_end() {
    let data = std::fs::read(MINI_MP4).expect("read minimal.mp4");

    let mut file = data;
    append_zero_sized_box(&mut file, b"xtra", b"proprietary extension data");

    let context = read_mp4(&file).expect("zero-sized unknown box should parse");
    assert_eq!(context.tracks.len(), 2);
}

#[test]
fn zero_sized_free_box_at_end() {
    let data = std::fs::read(MINI_MP4).expect("read minimal.mp4");

    let mut file = data;
    append_zero_sized_box(&mut file, b"free", b"");

    let context = read_mp4(&file).expect("zero-sized free box should parse");
    assert_eq!(context.tracks.len(), 2);
}

#[test]
fn zero_sized_moov_at_end() {
    let data = std::fs::read(MINI_MP4).expect("read minimal.mp4");
    let boxes = split_boxes(&data);
    let [ftyp, moov, ..] = &boxes[..] else {
        panic!("unexpected box layout in minimal.mp4")
    };
    assert_eq!(&moov[4..8], b"moov");

    let mut file = Vec::new();
    file.extend_from_slice(ftyp);
    append_zero_sized_box(&mut file, b"moov", &moov[8..]);

    let context = read_mp4(&file).expect("zero-sized moov should parse");
    assert_eq!(context.tracks.len(), 2);
}

#[test]
fn zero_sized_box_is_last_box() {
    // A zero-sized box extends to the end of the file, so anything after it
    // is (by definition) part of its content. A moov following a zero-sized
    // mdat is therefore unreachable.
    let data = std::fs::read(MINI_MP4).expect("read minimal.mp4");
    let boxes = split_boxes(&data);
    let [ftyp, moov, _free, mdat] = &boxes[..] else {
        panic!("unexpected box layout in minimal.mp4")
    };

    let mut file = Vec::new();
    file.extend_from_slice(ftyp);
    append_zero_sized_box(&mut file, b"mdat", &mdat[8..]);
    file.extend_from_slice(moov);

    assert!(matches!(read_mp4(&file), Err(mp4::Error::MoovMissing)));
}
