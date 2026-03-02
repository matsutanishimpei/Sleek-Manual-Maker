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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use app::RecorderApp;
use recorder::save_capture_and_log;
use types::{CaptureData, CaptureMessage, DisplayInfo};

/// マウス座標(f64, f64) を AtomicU64 に pack（x/y を i32 に丸めて上位/下位 32bit に格納）
#[inline]
fn pack_mouse(x: f64, y: f64) -> u64 {
    let xi = (x as i32) as u32 as u64;
    let yi = (y as i32) as u32 as u64;
    (xi << 32) | yi
}

/// AtomicU64 から (f64, f64) に unpack
#[inline]
fn unpack_mouse(v: u64) -> (f64, f64) {
    let x = (v >> 32) as u32 as i32 as f64;
    let y = (v as u32) as i32 as f64;
    (x, y)
}

fn main() -> Result<()> {
    // recordsディレクトリの作成
    fs::create_dir_all("records")?;

    let is_recording = Arc::new(AtomicBool::new(false));
    let is_recording_for_event = Arc::clone(&is_recording);
    let is_recording_for_gui   = Arc::clone(&is_recording);

    let (log_sender, log_receiver) = mpsc::channel();
    let log_sender_for_event = log_sender.clone();

    let image_counter = Arc::new(Mutex::new(0usize));
    let image_counter_for_event = Arc::clone(&image_counter);
    let image_counter_for_gui   = Arc::clone(&image_counter);

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
                    scale_factor: info.scale_factor,
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
    let current_session_folder_for_event = Arc::clone(&current_session_folder);
    let current_session_folder_for_gui   = Arc::clone(&current_session_folder);

    // バックグラウンド保存スレッド（Arc<Mutex>不要：スレッドが1つだけなのでmoveで直接渡せる）
    // 異常時にメモリが無尽蔵に増えるのを防ぐため、最大10フレームまでキューイングする sync_channel を使用
    let (bg_sender, bg_receiver) = mpsc::sync_channel::<CaptureMessage>(10);
    let log_sender_for_bg = log_sender.clone();
    
    thread::spawn(move || {
        while let Ok(msg) = bg_receiver.recv() {
            if let Err(e) = save_capture_and_log(msg) {
                let err_msg = format!("❌ 画像の保存に失敗しました（容量不足など）: {}", e);
                eprintln!("{}", err_msg);
                let _ = log_sender_for_bg.send(err_msg);
            }
        }
    });

    // イベント監視スレッド
    {
        let display_infos = Arc::clone(&display_infos);
        let cached_screens = Arc::clone(&cached_screens);
        let is_recording = Arc::clone(&is_recording_for_event);
        let log_sender = log_sender_for_event;
        let image_counter = Arc::clone(&image_counter_for_event);
        let current_session_folder = Arc::clone(&current_session_folder_for_event);

        thread::spawn(move || {
            // Mutex ではなく AtomicU64 で lock-free にマウス座標を共有
            let last_mouse_pos = Arc::new(AtomicU64::new(0));
            let last_mouse_pos_clone = Arc::clone(&last_mouse_pos);

            if let Err(e) = listen(move |event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    last_mouse_pos_clone.store(pack_mouse(x, y), Ordering::Relaxed);
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
                    Some(unpack_mouse(last_mouse_pos_clone.load(Ordering::Relaxed)))
                } else {
                    None
                };

                let timestamp = Local::now().to_rfc3339();

                // ディスプレイの特定（包含判定 -> 最近働判定）
                let target_display = if let Some((x, y)) = mouse_pos {
                    let mut found = display_infos
                        .iter()
                        .find(|d| d.contains_point(x, y))
                        .cloned();

                    #[cfg(debug_assertions)]
                    if let Some(ref d) = found {
                        log_sender.send(format!(
                            "[DBG検出] rdev({:.0},{:.0}) → Monitor ID:{} x:{} y:{} scale:{:.2}",
                            x, y, d.id, d.x, d.y, d.scale_factor
                        )).ok();
                    }

                    if found.is_none() {
                        #[cfg(debug_assertions)]
                        log_sender.send(format!(
                            "[DBG警告] rdev({:.0},{:.0}) が範囲外 → 最近働フォールバック", x, y
                        )).ok();

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
                            log_sender.send(format!(
                                "[DBGフォールバック] → Monitor ID:{} x:{} scale:{:.2}",
                                d.id, d.x, d.scale_factor
                            )).ok();
                        }
                    }
                    found
                } else {
                    log_sender
                        .send("[情報] マウス座標不明のため、メインディスプレイを使用します".to_string())
                        .ok();
                    display_infos.first().cloned()
                };

                // ?演算子でエラー処理をフラット化
                let capture_result: anyhow::Result<()> = (|| {
                    let target_display = target_display
                        .ok_or_else(|| anyhow::anyhow!("ディスプレイの特定に失敗: {}", action))?;

                    let screen = cached_screens
                        .iter()
                        .find(|s| s.display_info.id == target_display.id)
                        .ok_or_else(|| anyhow::anyhow!("対象ディスプレイが見つかりません: {}", action))?;

                    let screenshot = screen
                        .capture()
                        .map_err(|_| anyhow::anyhow!("スクリーンショットのキャプチャに失敗: {}", action))?;

                    // screenshots::Screen::capture() は RgbaImage を直接返す
                    let image_buffer = screenshot;

                    let session_folder = current_session_folder
                        .lock()
                        .unwrap()
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("セッションフォルダが未設定: {}", action))?;

                    let mut counter = image_counter.lock().unwrap();
                    *counter += 1;
                    let image_index = *counter;
                    drop(counter);

                    let msg = CaptureMessage {
                        capture: CaptureData {
                            display_info: target_display,
                            image_buffer,
                        },
                        mouse_pos: mouse_pos.unwrap_or((0.0, 0.0)),
                        has_mouse_pos: mouse_pos.is_some(),
                        timestamp: timestamp.clone(),
                        action: action.clone(),
                        session_folder,
                        image_index,
                    };

                    // キューが一杯ならスキップ（OSのフックやメモリをフリーズさせないため）
                    bg_sender
                        .try_send(msg)
                        .map_err(|_| anyhow::anyhow!("【警告】保存処理が追いついていません。このクリックはスキップされました: {}", action))?;

                    log_sender
                        .send(format!("[{}] {} を記録", timestamp, action))
                        .ok();

                    Ok(())
                })();

                if let Err(e) = capture_result {
                    log_sender.send(format!("[エラー] {}", e)).ok();
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
            *current_session_folder_for_gui.lock().unwrap() = Some(folder);
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
                is_recording_for_gui,
                log_receiver,
                image_counter_for_gui,
                session_sender,
            );

            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI起動エラー: {}", e))?;

    Ok(())
}
