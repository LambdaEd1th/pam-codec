use serde::{Deserialize, Serialize};

pub const PAM_MAGIC: u32 = 0xBAF01954;

#[derive(Debug, Serialize, Deserialize)]
pub struct PamInfo {
    pub version: i32,
    pub frame_rate: i32,
    pub position: [f64; 2],
    pub size: [f64; 2],
    pub image: Vec<ImageInfo>,
    pub sprite: Vec<SpriteInfo>,
    pub main_sprite: SpriteInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    pub size: [i32; 2],
    pub transform: Vec<f64>, // Using Vec because length varies
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SpriteInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub frame_rate: f64,
    pub work_area: [i32; 2],
    pub frame: Vec<FrameInfo>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FrameInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub stop: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<[String; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<RemovesInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub append: Vec<AddsInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub change: Vec<MovesInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemovesInfo {
    pub index: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddsInfo {
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Image or sprite resource index (wire format: u8, extended to u16 in v6+).
    pub resource: u32,
    pub sprite: bool,
    pub additive: bool,
    /// Preload frame number (wire format: u16).
    pub preload_frame: u32,
    pub time_scale: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovesInfo {
    pub index: i32,
    pub transform: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_rectangle: Option<[i32; 4]>,
    /// Sprite frame number (wire format: u16, max 65535).
    pub sprite_frame_number: u32,
}

bitflags::bitflags! {
    pub struct FrameFlags: u8 {
        const REMOVES = 1;
        const ADDS = 2;
        const MOVES = 4;
        const FRAME_NAME = 8;
        const STOP = 16;
        const COMMANDS = 32;
    }
}

bitflags::bitflags! {
    pub struct MoveFlags: u16 {
        const SRC_RECT = 32768;
        const ROTATE = 16384;
        const COLOR = 8192;
        const MATRIX = 4096;
        const LONG_COORDS = 2048;
        const ANIM_FRAME_NUM = 1024;
    }
}
