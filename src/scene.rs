//! Compact scene format for share links: a header, then run-length encoded elements.
//!
//! `S B 1 | width: u16 LE | height: u16 LE | (element: u8, run: LEB128)*`
//!
//! A typical drawing is mostly empty space and long runs of the same material, so it compresses to a
//! few hundred bytes: small enough for a URL.

use crate::element::Element;
use crate::world::World;

const MAGIC: &[u8; 3] = b"SB1";
/// Upper bound on scene size when decoding untrusted input.
pub const MAX_SIDE: usize = 1024;

pub fn encode(world: &World) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(world.width() as u16).to_le_bytes());
    out.extend_from_slice(&(world.height() as u16).to_le_bytes());
    let mut current: Option<(Element, u64)> = None;
    for y in 0..world.height() {
        for x in 0..world.width() {
            let element = world.get(x, y);
            current = match current {
                Some((e, n)) if e == element => Some((e, n + 1)),
                Some((e, n)) => {
                    push_run(&mut out, e, n);
                    Some((element, 1))
                }
                None => Some((element, 1)),
            };
        }
    }
    if let Some((e, n)) = current {
        push_run(&mut out, e, n);
    }
    out
}

fn push_run(out: &mut Vec<u8>, element: Element, mut run: u64) {
    out.push(element as u8);
    loop {
        let byte = (run & 0x7f) as u8;
        run >>= 7;
        if run == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Why a scene could not be read.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    NotAScene,
    BadSize,
    Corrupt,
}

pub fn decode(bytes: &[u8], seed: u32) -> Result<World, DecodeError> {
    if bytes.len() < 7 || &bytes[..3] != MAGIC {
        return Err(DecodeError::NotAScene);
    }
    let width = u16::from_le_bytes([bytes[3], bytes[4]]) as usize;
    let height = u16::from_le_bytes([bytes[5], bytes[6]]) as usize;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(DecodeError::BadSize);
    }
    let total = width * height;
    let mut world = World::new(width, height, seed);
    let mut position = 0usize;
    let mut i = 7;
    while i < bytes.len() {
        let element = Element::from_u8(bytes[i]).ok_or(DecodeError::Corrupt)?;
        i += 1;
        let mut run: u64 = 0;
        let mut shift = 0;
        loop {
            let byte = *bytes.get(i).ok_or(DecodeError::Corrupt)?;
            i += 1;
            if shift > 35 {
                return Err(DecodeError::Corrupt);
            }
            run |= ((byte & 0x7f) as u64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        let end = position
            .checked_add(run as usize)
            .ok_or(DecodeError::Corrupt)?;
        if end > total {
            return Err(DecodeError::Corrupt);
        }
        if element != Element::Empty {
            for p in position..end {
                world.set(p % width, p / width, element);
            }
        }
        position = end;
    }
    if position != total {
        return Err(DecodeError::Corrupt);
    }
    world.render();
    Ok(world)
}
