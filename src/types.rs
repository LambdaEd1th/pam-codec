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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_sprite: Option<SpriteInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[i32; 2]>,
    pub transform: Vec<f64>, // Using Vec because length varies
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SpriteInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_area: Option<[i32; 2]>,
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
    #[serde(default)]
    pub sprite: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub additive: bool,
    /// Preload frame number (wire format: signed i16 when present).
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub preload_frame: i32,
    #[serde(default = "default_time_scale", skip_serializing_if = "is_one_f32")]
    pub time_scale: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MovesInfo {
    pub index: i32,
    pub transform: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_rectangle: Option<[f64; 4]>,
    /// Sprite frame number (wire format: signed i16 when present).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_frame_number: Option<i32>,
}

fn default_time_scale() -> f32 {
    1.0
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

fn is_one_f32(value: &f32) -> bool {
    (*value - 1.0).abs() < f32::EPSILON
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
