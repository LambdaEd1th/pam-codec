use crate::error::Result;
use crate::types::*;
use byteorder::{LE, WriteBytesExt};
use std::io::Write;

pub fn encode_pam<W: Write>(pam: &PamInfo, writer: &mut W) -> Result<()> {
    writer.write_u32::<LE>(PAM_MAGIC)?;
    writer.write_i32::<LE>(pam.version)?;
    writer.write_u8(pam.frame_rate as u8)?;

    for p in &pam.position {
        writer.write_u16::<LE>((p * 20.0) as u16)?;
    }

    for s in &pam.size {
        writer.write_u16::<LE>((s * 20.0) as u16)?;
    }

    writer.write_u16::<LE>(pam.image.len() as u16)?;
    for img in &pam.image {
        write_image_info(img, writer, pam.version)?;
    }

    writer.write_u16::<LE>(pam.sprite.len() as u16)?;
    for sprite in &pam.sprite {
        write_sprite_info(sprite, writer, pam.version)?;
    }

    if pam.version > 3 {
        // Decoder reads a bool: false = no main sprite, true = main sprite follows.
        // If the main_sprite has no frames (and no meaningful data), write false
        // to preserve round-trip fidelity with files that lack a main sprite.
        let has_main = !pam.main_sprite.frame.is_empty()
            || pam.main_sprite.name.is_some()
            || pam.main_sprite.description.is_some();
        writer.write_u8(if has_main { 1 } else { 0 })?;
        if has_main {
            write_sprite_info(&pam.main_sprite, writer, pam.version)?;
        }
    } else {
        // Version <= 3 always has main sprite implicit
        write_sprite_info(&pam.main_sprite, writer, pam.version)?;
    }

    Ok(())
}

fn write_string_by_u16<W: Write>(s: &str, writer: &mut W) -> Result<()> {
    let bytes = s.as_bytes();
    writer.write_u16::<LE>(bytes.len() as u16)?;
    writer.write_all(bytes)?;
    Ok(())
}

fn write_image_info<W: Write>(img: &ImageInfo, writer: &mut W, version: i32) -> Result<()> {
    write_string_by_u16(&img.name, writer)?;

    // PAM image transforms always have 6 elements (a,b,c,d,tx,ty).
    debug_assert!(
        img.transform.len() == 6 || img.transform.is_empty(),
        "ImageInfo transform must have 6 elements or be empty, got {}",
        img.transform.len()
    );

    if version >= 4 {
        for s in &img.size {
            writer.write_u16::<LE>(*s as u16)?;
        }
    }

    if version == 1 {
        // v1 stores: angle (u16/1000) → cos, sin, -sin, cos matrix.
        // Use atan2(sin, cos) to recover the signed angle — preserves
        // rotation direction, unlike acos which loses the sign.
        let val = if img.transform.len() >= 2 {
            f64::atan2(img.transform[1], img.transform[0]) * 1000.0
        } else {
            0.0
        };
        writer.write_u16::<LE>(val as u16)?;

        let tx = if img.transform.len() > 4 {
            img.transform[4]
        } else {
            0.0
        };
        let ty = if img.transform.len() > 5 {
            img.transform[5]
        } else {
            0.0
        };
        writer.write_i16::<LE>((tx * 20.0) as i16)?;
        writer.write_i16::<LE>((ty * 20.0) as i16)?;
    } else {
        // t[0] = read_i32 / 1310720.0
        let t = &img.transform;
        let c = 1310720.0;
        let get = |i| {
            if i < t.len() {
                t[i]
            } else if i % 4 == 0 || i % 4 == 3 {
                1.0
            } else {
                0.0
            }
        };

        writer.write_i32::<LE>((get(0) * c) as i32)?; // a
        writer.write_i32::<LE>((get(2) * c) as i32)?; // c (stored as second int in decode)
        writer.write_i32::<LE>((get(1) * c) as i32)?; // b
        writer.write_i32::<LE>((get(3) * c) as i32)?; // d

        // tx, ty
        let tx = if t.len() > 4 { t[4] } else { 0.0 };
        let ty = if t.len() > 5 { t[5] } else { 0.0 };
        writer.write_i16::<LE>((tx * 20.0) as i16)?;
        writer.write_i16::<LE>((ty * 20.0) as i16)?;
    }

    Ok(())
}

fn write_sprite_info<W: Write>(sprite: &SpriteInfo, writer: &mut W, version: i32) -> Result<()> {
    if version >= 4 {
        write_string_by_u16(sprite.name.as_deref().unwrap_or(""), writer)?;
        if version >= 6 {
            write_string_by_u16(sprite.description.as_deref().unwrap_or(""), writer)?;
        }
        writer.write_i32::<LE>((sprite.frame_rate * 65536.0) as i32)?;
    }

    // frames_count
    writer.write_u16::<LE>(sprite.frame.len() as u16)?;

    if version >= 5 {
        writer.write_u16::<LE>(sprite.work_area[0] as u16)?;
        writer.write_u16::<LE>(sprite.work_area[1] as u16)?;
    } else {
        // Implicit work_area logic in decode, nothing to write.
    }

    for frame in &sprite.frame {
        write_frame_info(frame, writer, version)?;
    }

    Ok(())
}

fn write_frame_info<W: Write>(frame: &FrameInfo, writer: &mut W, version: i32) -> Result<()> {
    let mut flags = FrameFlags::empty();
    if !frame.remove.is_empty() {
        flags |= FrameFlags::REMOVES;
    }
    if !frame.append.is_empty() {
        flags |= FrameFlags::ADDS;
    }
    if !frame.change.is_empty() {
        flags |= FrameFlags::MOVES;
    }
    if frame.label.is_some() {
        flags |= FrameFlags::FRAME_NAME;
    }
    if frame.stop {
        flags |= FrameFlags::STOP;
    }
    if !frame.command.is_empty() {
        flags |= FrameFlags::COMMANDS;
    }

    writer.write_u8(flags.bits())?;

    if flags.contains(FrameFlags::REMOVES) {
        let count = frame.remove.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for rem in &frame.remove {
            write_removes_info(rem, writer)?;
        }
    }

    if flags.contains(FrameFlags::ADDS) {
        let count = frame.append.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for add in &frame.append {
            write_adds_info(add, writer, version)?;
        }
    }

    if flags.contains(FrameFlags::MOVES) {
        let count = frame.change.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for change in &frame.change {
            write_moves_info(change, writer, version)?;
        }
    }

    if let Some(label) = &frame.label {
        write_string_by_u16(label, writer)?;
    }

    if flags.contains(FrameFlags::COMMANDS) {
        let count = frame.command.len();
        if count >= 255 {
            writer.write_u8(255)?;
            writer.write_u16::<LE>(count as u16)?;
        } else {
            writer.write_u8(count as u8)?;
        }
        for cmd in &frame.command {
            write_string_by_u16(&cmd[0], writer)?;
            write_string_by_u16(&cmd[1], writer)?;
        }
    }

    Ok(())
}

fn write_removes_info<W: Write>(info: &RemovesInfo, writer: &mut W) -> Result<()> {
    if info.index >= 2047 {
        // Technically read logic: read_u16, if >= 2047 then read_i32.
        // We need to encode such that decode reads it back.
        // But 2047 barely fits in u16.
        // Logic: val = read_u16; if val >= 2047 { val = read_i32 }
        // So to write large index, we write 2047 (u16) then the actual index (i32).
        writer.write_u16::<LE>(2047)?;
        writer.write_i32::<LE>(info.index)?;
    } else {
        writer.write_u16::<LE>(info.index as u16)?;
    }
    Ok(())
}

fn write_adds_info<W: Write>(info: &AddsInfo, writer: &mut W, version: i32) -> Result<()> {
    // num encoding
    // num & 2047 = index
    // num & 32768 = sprite (bool)
    // num & 16384 = additive (bool)
    // num & 8192 = has preload_frame
    // num & 4096 = has name
    // num & 2048 = has time_scale

    let mut num = 0u16;
    let large_index = info.index >= 2047;

    if large_index {
        num |= 2047;
    } else {
        num |= info.index as u16;
    }

    if info.sprite {
        num |= 32768;
    }
    if info.additive {
        num |= 16384;
    }
    if info.preload_frame > 0 {
        num |= 8192;
    } // Logic check: read only reads if flag set
    // What if preload_frame is 0? decode: if flag set read, else 0.
    // So if 0, we don't set flag.

    if info.name.is_some() {
        num |= 4096;
    }
    if (info.time_scale - 1.0).abs() > 0.0001 {
        num |= 2048;
    }

    writer.write_u16::<LE>(num)?;

    if large_index {
        writer.write_i32::<LE>(info.index)?;
    }

    // Resource
    if version >= 6 && info.resource >= 255 {
        writer.write_u8(255)?;
        writer.write_u16::<LE>(info.resource as u16)?;
    } else {
        writer.write_u8((info.resource & 0xFF) as u8)?;
    }

    if (num & 8192) != 0 {
        writer.write_u16::<LE>(info.preload_frame as u16)?;
    }

    if let Some(name) = &info.name {
        write_string_by_u16(name, writer)?;
    }

    if (num & 2048) != 0 {
        writer.write_i32::<LE>((info.time_scale * 65536.0) as i32)?;
    }

    Ok(())
}

fn write_moves_info<W: Write>(info: &MovesInfo, writer: &mut W, _version: i32) -> Result<()> {
    // num7 encoding
    // num & 1023 = index
    // Flags

    let mut num = 0u16;
    let large_index = info.index >= 1023;
    if large_index {
        num |= 1023;
    } else {
        num |= info.index as u16;
    }

    // Detect flags based on data presence
    // Decoding: if MATRIX read 4 i32s, else if ROTATE read 1 i16, else 0.
    // Then read coords.

    // We need to compare transform against identity/defaults to be minimal?
    // Or just respect what's in the vector.
    // If we have 6 elements, use MATRIX.
    // If 3 elements (idx 0 only used), use ROTATE?
    // But standard MovesInfo struct stores everything in a Vec.

    // Logic:
    // If transform.len() >= 6 (a,b,c,d,tx,ty) -> Matrix (tx,ty separate)
    // Actually decode puts tx,ty at end of vec.
    // MATRIX reads indices 0,2,1,3.
    // ROTATE reads index 0.
    // Then tx,ty are read (LONG_COORDS check).

    let mut flags = MoveFlags::empty();

    if info.transform.len() >= 6 {
        flags |= MoveFlags::MATRIX;
    } else if info.transform.len() >= 3 && info.transform[0].abs() > f64::EPSILON {
        // Use f64::EPSILON instead of a hardcoded 0.00001 to avoid silently
        // dropping near-zero rotations.
        flags |= MoveFlags::ROTATE;
    }

    if info.source_rectangle.is_some() {
        flags |= MoveFlags::SRC_RECT;
    }
    if info.color.is_some() {
        flags |= MoveFlags::COLOR;
    }
    if info.sprite_frame_number > 0 {
        flags |= MoveFlags::ANIM_FRAME_NUM;
    }

    // Determine coords type based on transform values
    // LONG_COORDS if either coord exceeds i16 range (in 1/20 units)
    let len = info.transform.len();
    let tx_raw = if len >= 2 { info.transform[len - 2] } else { 0.0 };
    let ty_raw = if len >= 2 { info.transform[len - 1] } else { 0.0 };
    let tx_i16 = (tx_raw * 20.0) as i16;
    let ty_i16 = (ty_raw * 20.0) as i16;
    if (tx_i16 as f64 / 20.0 - tx_raw).abs() > 0.001
        || (ty_i16 as f64 / 20.0 - ty_raw).abs() > 0.001
    {
        flags |= MoveFlags::LONG_COORDS;
    }

    num |= flags.bits();

    writer.write_u16::<LE>(num)?;

    if large_index {
        writer.write_i32::<LE>(info.index)?;
    }

    if flags.contains(MoveFlags::MATRIX) {
        writer.write_i32::<LE>((info.transform[0] * 65536.0) as i32)?; // a
        writer.write_i32::<LE>((info.transform[2] * 65536.0) as i32)?; // c
        writer.write_i32::<LE>((info.transform[1] * 65536.0) as i32)?; // b
        writer.write_i32::<LE>((info.transform[3] * 65536.0) as i32)?; // d
    } else if flags.contains(MoveFlags::ROTATE) {
        writer.write_i16::<LE>((info.transform[0] * 1000.0) as i16)?;
    }

    if flags.contains(MoveFlags::LONG_COORDS) {
        writer.write_i32::<LE>((tx_raw * 20.0) as i32)?;
        writer.write_i32::<LE>((ty_raw * 20.0) as i32)?;
    } else {
        writer.write_i16::<LE>((tx_raw * 20.0) as i16)?;
        writer.write_i16::<LE>((ty_raw * 20.0) as i16)?;
    }

    if flags.contains(MoveFlags::SRC_RECT) {
        if let Some(sr) = &info.source_rectangle {
            for v in sr {
                writer.write_i16::<LE>(*v as i16 * 20)?;
            }
        }
    }

    if flags.contains(MoveFlags::COLOR) {
        if let Some(c) = &info.color {
            for v in c {
                writer.write_u8((v * 255.0) as u8)?;
            }
        }
    }

    if flags.contains(MoveFlags::ANIM_FRAME_NUM) {
        writer.write_u16::<LE>(info.sprite_frame_number as u16)?;
    }

    Ok(())
}
