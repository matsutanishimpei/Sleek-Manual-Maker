use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ディスプレイ情報のキャッシュ用構造体
#[derive(Clone, Debug)]
pub struct DisplayInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub id: u32,
}

impl DisplayInfo {
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        let x = x as i32;
        let y = y as i32;
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    // 点からディスプレイ矩形までの距離（の2乗）を計算
    pub fn distance_sq(&self, x: f64, y: f64) -> f64 {
        let min_x = self.x as f64;
        let max_x = (self.x + self.width as i32) as f64;
        let min_y = self.y as f64;
        let max_y = (self.y + self.height as i32) as f64;

        let dx = if x < min_x {
            min_x - x
        } else if x > max_x {
            x - max_x
        } else {
            0.0
        };

        let dy = if y < min_y {
            min_y - y
        } else if y > max_y {
            y - max_y
        } else {
            0.0
        };

        dx * dx + dy * dy
    }
}

// キャプチャデータ
pub struct CaptureData {
    pub display_info: DisplayInfo,
    pub image_buffer: RgbaImage,
}

// 操作ログエントリ
#[derive(Serialize, Deserialize, Clone)]
pub struct OperationLog {
    pub timestamp: String,
    pub action: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub target_monitor_id: u32,
    pub image_path: String,
    // 画像サイズ（CSSオーバーレイ用、古いログとの互換性のためにOptionかつdefault）
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

// バックグラウンド処理へ送るメッセージ
pub struct CaptureMessage {
    pub capture: CaptureData,
    pub mouse_pos: (f64, f64),
    pub timestamp: String,
    pub action: String,
    pub session_folder: PathBuf,
    pub image_index: usize,
}

// アプリケーション状態
#[derive(Clone, PartialEq)]
pub enum AppState {
    Idle,
    Recording,
    Review,
}
