use crate::types::{CaptureMessage, OperationLog};
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;

pub fn save_capture_and_log(msg: CaptureMessage) -> Result<()> {
    let CaptureMessage {
        capture,
        mouse_pos,
        has_mouse_pos,
        timestamp,
        action,
        session_folder,
        image_index,
    } = msg;

    // 画像ファイル名
    let filename = format!("image_{:03}.png", image_index);
    let image_path = session_folder.join(&filename);

    // 画像保存
    capture.image_buffer.save(&image_path)?;

    // マウス座標を、対象ディスプレイ（画像）内の物理ピクセル相対座標に変換する
    // has_mouse_pos フラグで座標の有無を明示的に判定（原点クリックでも正しく動作）
    //
    // 座標系の整理（screenshots crateの挙動を実測で確認済み）:
    //   rdev mouse_pos    : 物理ピクセル座標
    //   display_info.x/y  : 物理ピクセル座標（仮想デスクトップ上の位置）
    //   display_info.w/h  : 論理ピクセル（= 物理 / scale_factor）
    //   capture画像       : 物理ピクセル（image_buffer.width/height()で取得）
    //
    // → 相対座標 = rdev - display.x/y（共に物理、変換不要）
    let (rel_x, rel_y) = if has_mouse_pos {
        let rx = (mouse_pos.0 - capture.display_info.x as f64).round() as i32;
        let ry = (mouse_pos.1 - capture.display_info.y as f64).round() as i32;
        (Some(rx), Some(ry))
    } else {
        (None, None)
    };

    // 画像の実際の物理ピクセルサイズを取得（display_info.w/hは論理なので使わない）
    let img_physical_w = capture.image_buffer.width();
    let img_physical_h = capture.image_buffer.height();

    let log_entry = OperationLog {
        timestamp,
        action,
        x: rel_x,
        y: rel_y,
        target_monitor_id: capture.display_info.id,
        image_path: filename,
        width: Some(img_physical_w),
        height: Some(img_physical_h),
    };

    let log_path = session_folder.join("session_log.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    writeln!(file, "{}", serde_json::to_string(&log_entry)?)?;

    Ok(())
}
