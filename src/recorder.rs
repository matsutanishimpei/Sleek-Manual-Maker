use crate::types::{CaptureMessage, OperationLog};
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;

pub fn save_capture_and_log(msg: CaptureMessage) -> Result<()> {
    let CaptureMessage {
        capture,
        mouse_pos,
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

    // JSONLログ記録
    // マウス座標を、対象ディスプレイ（画像）内の相対座標に変換する
    let (rel_x, rel_y) = if mouse_pos != (0.0, 0.0) {
        let rx = mouse_pos.0 as i32 - capture.display_info.x;
        let ry = mouse_pos.1 as i32 - capture.display_info.y;
        (Some(rx), Some(ry))
    } else {
        (None, None)
    };

    let log_entry = OperationLog {
        timestamp,
        action,
        x: rel_x,
        y: rel_y,
        target_monitor_id: capture.display_info.id,
        image_path: filename,
        width: Some(capture.display_info.width),
        height: Some(capture.display_info.height),
    };

    let log_path = session_folder.join("session_log.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    writeln!(file, "{}", serde_json::to_string(&log_entry)?)?;

    Ok(())
}
