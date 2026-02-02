#![windows_subsystem = "windows"]

mod app;
mod generator;
mod recorder;
mod types;

use anyhow::Result;
use chrono::Local;
use eframe::egui;
use rdev::{listen, Button, EventType, Key};
use screenshots::Screen;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use app::RecorderApp;
use recorder::save_capture_and_log;
use types::{CaptureData, CaptureMessage, DisplayInfo};

fn main() -> Result<()> {
    // recordsディレクトリの作成
    fs::create_dir_all("records")?;

    let is_recording = Arc::new(AtomicBool::new(false));
    let is_recording_clone = Arc::clone(&is_recording);

    let (log_sender, log_receiver) = mpsc::channel();
    let log_sender_clone = log_sender.clone();

    let image_counter = Arc::new(Mutex::new(0usize));
    let image_counter_clone = Arc::clone(&image_counter);

    // ディスプレイ情報を取得
    let screens = Screen::all().expect("Failed to get screen information");
    let display_infos: Arc<Vec<DisplayInfo>> = Arc::new(
        screens
            .iter()
            .map(|screen| {
                let info = &screen.display_info;
                DisplayInfo {
                    x: info.x,
                    y: info.y,
                    width: info.width,
                    height: info.height,
                    id: info.id,
                }
            })
            .collect(),
    );

    // 起動ログ・モニター情報出力
    log_sender.send("[INFO] Application started.".to_string()).ok();
    log_sender.send(format!("[INFO] Detected {} monitors:", screens.len())).ok();

    for (i, screen) in screens.iter().enumerate() {
        let info = &screen.display_info;
        log_sender
            .send(format!(
                "[INFO] - Monitor {} (ID:{}): {}x{} at ({},{})",
                i, info.id, info.width, info.height, info.x, info.y
            ))
            .ok();
    }

    let cached_screens = Arc::new(screens);

    log_sender
        .send("準備完了！録画開始ボタンを押してください。".to_string())
        .ok();

    // セッションフォルダパスの共有
    let current_session_folder = Arc::new(Mutex::new(None::<PathBuf>));
    let current_session_folder_clone = Arc::clone(&current_session_folder);

    // バックグラウンド保存スレッド
    let (bg_sender, bg_receiver) = mpsc::channel::<CaptureMessage>();
    let bg_receiver = Arc::new(Mutex::new(bg_receiver));

    {
        let bg_receiver = Arc::clone(&bg_receiver);
        thread::spawn(move || {
            while let Ok(msg) = bg_receiver.lock().unwrap().recv() {
                if let Err(e) = save_capture_and_log(msg) {
                    eprintln!("保存エラー: {}", e);
                }
            }
        });
    }

    // イベント監視スレッド
    {
        let display_infos = Arc::clone(&display_infos);
        let cached_screens = Arc::clone(&cached_screens);
        let is_recording = Arc::clone(&is_recording_clone);
        let log_sender = log_sender_clone;
        let image_counter = Arc::clone(&image_counter_clone);
        let current_session_folder = Arc::clone(&current_session_folder_clone);

        thread::spawn(move || {
            let last_mouse_pos = Arc::new(Mutex::new((0.0, 0.0)));
            let last_mouse_pos_clone = Arc::clone(&last_mouse_pos);

            if let Err(e) = listen(move |event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    if let Ok(mut pos) = last_mouse_pos_clone.lock() {
                        *pos = (x, y);
                    }
                }

                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }

                let (action, needs_mouse_pos) = match event.event_type {
                    EventType::ButtonPress(Button::Left) => ("MouseClick".to_string(), true),
                    EventType::ButtonPress(Button::Right) => ("RightClick".to_string(), true),
                    EventType::KeyPress(Key::Tab) => ("KeyPress_Tab".to_string(), false),
                    _ => return,
                };

                let mouse_pos = if needs_mouse_pos {
                    last_mouse_pos_clone.lock().ok().map(|pos| *pos)
                } else {
                    None
                };

                let timestamp = Local::now().to_rfc3339();

                // ディスプレイの特定（包含判定 -> 最近傍判定）
                let target_display = if let Some((x, y)) = mouse_pos {
                    // まず、座標を含むディスプレイを探す
                    let mut found = display_infos
                        .iter()
                        .find(|d| d.contains_point(x, y))
                        .cloned();

                    // 見つからない場合、最も近いディスプレイを探す (High-DPI対策)
                    if found.is_none() {
                        // High-DPI環境では頻繁に発生するため、Debugレベルのみで記録
                        #[cfg(debug_assertions)]
                        log_sender
                            .send(format!("[デバッグ] 座標 ({:.1}, {:.1}) は範囲外です。High-DPI補正として最近傍ディスプレイを検索します。", x, y))
                            .ok();

                        found = display_infos
                            .iter()
                            .min_by(|a, b| {
                                a.distance_sq(x, y)
                                    .partial_cmp(&b.distance_sq(x, y))
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .cloned();

                        #[cfg(debug_assertions)]
                        if let Some(ref d) = found {
                            log_sender
                                .send(format!(
                                    "[デバッグ] High-DPI adjustment: Coordinate ({:.1}, {:.1}) mapped to Screen ID {}",
                                    x, y, d.id
                                ))
                                .ok();
                        }
                    }
                    found
                } else {
                    log_sender
                        .send(format!(
                            "[情報] マウス座標不明のため、メインディスプレイを使用します"
                        ))
                        .ok();
                    display_infos.first().cloned()
                };

                if let Some(target_display) = target_display {
                    if let Some(screen) = cached_screens
                        .iter()
                        .find(|s| s.display_info.id == target_display.id)
                    {
                        if let Ok(screenshot) = screen.capture() {
                            let temp_path =
                                format!("temp_screenshot_{}.png", target_display.id);
                            if screenshot.save(&temp_path).is_ok() {
                                if let Ok(dynamic_image) = image::open(&temp_path) {
                                    let image_buffer = dynamic_image.to_rgba8();
                                    let _ = fs::remove_file(&temp_path);

                                    let session_folder =
                                        current_session_folder.lock().unwrap().clone();

                                    if let Some(session_folder) = session_folder {
                                        let mut counter = image_counter.lock().unwrap();
                                        *counter += 1;
                                        let image_index = *counter;

                                        let capture_data = CaptureData {
                                            display_info: target_display.clone(),
                                            image_buffer,
                                        };

                                        let msg = CaptureMessage {
                                            capture: capture_data,
                                            mouse_pos: mouse_pos.unwrap_or((0.0, 0.0)),
                                            timestamp: timestamp.clone(),
                                            action: action.clone(),
                                            session_folder,
                                            image_index,
                                        };
                                        if bg_sender.send(msg).is_ok() {
                                            log_sender
                                                .send(format!("[{}] {} を記録", timestamp, action))
                                                .ok();
                                        } else {
                                            log_sender
                                                .send(format!(
                                                    "[エラー] 保存スレッドへの送信に失敗: {}",
                                                    action
                                                ))
                                                .ok();
                                        }
                                    } else {
                                        log_sender
                                            .send(format!(
                                                "[エラー] セッションフォルダが未設定: {}",
                                                action
                                            ))
                                            .ok();
                                    }
                                } else {
                                    log_sender
                                        .send(format!("[エラー] 画像の読み込みに失敗: {}", action))
                                        .ok();
                                }
                            } else {
                                log_sender
                                    .send(format!(
                                        "[エラー] スクリーンショットの保存に失敗: {}",
                                        action
                                    ))
                                    .ok();
                            }
                        } else {
                            log_sender
                                .send(format!(
                                    "[エラー] スクリーンショットのキャプチャに失敗: {}",
                                    action
                                ))
                                .ok();
                        }
                    } else {
                        log_sender
                            .send(format!(
                                "[エラー] 対象ディスプレイが見つかりません: {}",
                                action
                            ))
                            .ok();
                    }
                } else {
                    log_sender
                        .send(format!("[エラー] ディスプレイの特定に失敗: {}", action))
                        .ok();
                }
            }) {
                eprintln!("イベント監視エラー: {:?}", e);
            }
        });
    }

    // セッションフォルダ更新用のチャネル
    let (session_sender, session_receiver) = mpsc::channel::<PathBuf>();

    // GUIスレッドでセッションフォルダを更新
    std::thread::spawn(move || {
        while let Ok(folder) = session_receiver.recv() {
            *current_session_folder_clone.lock().unwrap() = Some(folder);
        }
    });

    // GUIアプリケーションを起動
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PC操作ロガー",
        options,
        Box::new(move |cc| {
            let app = RecorderApp::new(
                cc,
                is_recording_clone,
                log_receiver,
                image_counter_clone,
                session_sender,
            );

            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI起動エラー: {}", e))?;

    Ok(())
}
