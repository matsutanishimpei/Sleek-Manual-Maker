use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ディスプレイ情報のキャッシュ用構造体
#[derive(Clone, Debug)]
pub struct DisplayInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,      // 物理ピクセル（実際の画像の幅）
    pub height: u32,     // 物理ピクセル（実際の画像の高さ）
    pub scale_factor: f32, // DPIスケール（例: 125% → 1.25）
    pub id: u32,
}

impl DisplayInfo {
    /// rdev物理座標がこのディスプレイの物理範囲内に含まれるか判定
    /// display.x/y は物理座標、width/height は論理ピクセル
    /// 物理終端 = x + width * scale_factor
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        let x = x as i32;
        let y = y as i32;
        let physical_w = (self.width as f32 * self.scale_factor) as i32;
        let physical_h = (self.height as f32 * self.scale_factor) as i32;
        x >= self.x
            && x < self.x + physical_w
            && y >= self.y
            && y < self.y + physical_h
    }

    // 点からディスプレイ物理矩形までの距離（の2乗）を計算
    // 全て物理座標空間で比較
    pub fn distance_sq(&self, x: f64, y: f64) -> f64 {
        let min_x = self.x as f64;
        let physical_w = self.width as f64 * self.scale_factor as f64;
        let physical_h = self.height as f64 * self.scale_factor as f64;
        let max_x = min_x + physical_w;
        let min_y = self.y as f64;
        let max_y = min_y + physical_h;

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
    pub has_mouse_pos: bool,   // 座標の有無を明示（ゼロ値との区別のため）
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
