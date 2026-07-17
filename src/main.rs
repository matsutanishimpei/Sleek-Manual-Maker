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
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use app::RecorderApp;
use recorder::save_capture_and_log;
use types::{CaptureData, CaptureMessage, DisplayInfo, RecordTrigger};

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

fn decrement_pending_saves(pending_saves: &AtomicUsize) {
    let _ = pending_saves.fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
        Some(count.saturating_sub(1))
    });
}

fn write_startup_error(message: &str) {
    let _ = fs::create_dir_all("log");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("log/application.log")
    {
        let _ = writeln!(file, "{}", message);
    }
}

struct PendingWorkGuard<'a> {
    pending_saves: &'a AtomicUsize,
    active: bool,
}

impl<'a> PendingWorkGuard<'a> {
    fn new(pending_saves: &'a AtomicUsize) -> Self {
        pending_saves.fetch_add(1, Ordering::AcqRel);
        Self {
            pending_saves,
            active: true,
        }
    }

    fn dismiss(&mut self) {
        self.active = false;
    }
}

impl Drop for PendingWorkGuard<'_> {
    fn drop(&mut self) {
        if self.active {
            decrement_pending_saves(self.pending_saves);
        }
    }
}

#[cfg(target_os = "windows")]
fn get_active_window_title() -> String {
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return "不明なウィンドウ".to_string();
        }
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            "デスクトップ".to_string()
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_active_window_title() -> String {
    "不明なウィンドウ".to_string()
}

#[cfg(target_os = "windows")]
fn get_current_mouse_pos() -> (f64, f64) {
    use winapi::shared::windef::POINT;
    use winapi::um::winuser::GetCursorPos;
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            (pt.x as f64, pt.y as f64)
        } else {
            (0.0, 0.0)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_current_mouse_pos() -> (f64, f64) {
    (0.0, 0.0)
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

    let pending_saves = Arc::new(AtomicUsize::new(0));
    let pending_saves_for_event = Arc::clone(&pending_saves);
    let pending_saves_for_bg = Arc::clone(&pending_saves);
    let pending_saves_for_gui = Arc::clone(&pending_saves);

    // ディスプレイ情報を取得
    let screens = Screen::all().map_err(|e| {
        let message = format!("画面情報の取得に失敗しました: {}", e);
        write_startup_error(&message);
        anyhow::anyhow!(message)
    })?;
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

    // バックグラウンドキャプチャ＆保存スレッド
    // 異常時にメモリが無尽蔵に増えるのを防ぐため、最大10フレームまでキューイングする sync_channel を使用
    let (bg_sender, bg_receiver) = mpsc::sync_channel::<RecordTrigger>(10);
    let log_sender_for_bg = log_sender.clone();
    let cached_screens_for_bg = Arc::clone(&cached_screens);
    
    thread::spawn(move || {
        while let Ok(trigger) = bg_receiver.recv() {
            let result: Result<()> = (|| {
                let screen = cached_screens_for_bg
                    .iter()
                    .find(|s| s.display_info.id == trigger.target_display.id)
                    .ok_or_else(|| anyhow::anyhow!("対象ディスプレイが見つかりません"))?;

                let screenshot = screen
                    .capture()
                    .map_err(|_| anyhow::anyhow!("スクリーンショットのキャプチャに失敗"))?;

                let msg = CaptureMessage {
                    capture: CaptureData {
                        display_info: trigger.target_display,
                        image_buffer: screenshot,
                    },
                    mouse_pos: trigger.mouse_pos,
                    has_mouse_pos: trigger.has_mouse_pos,
                    timestamp: trigger.timestamp,
                    action: trigger.action,
                    session_folder: trigger.session_folder,
                    image_index: trigger.image_index,
                    window_title: trigger.window_title,
                };

                save_capture_and_log(msg)?;
                Ok(())
            })();

            if let Err(e) = result {
                let err_msg = format!("❌ 録画データの保存に失敗しました: {}", e);
                eprintln!("{}", err_msg);
                let _ = log_sender_for_bg.send(err_msg);
            }

            decrement_pending_saves(&pending_saves_for_bg);
        }
    });

    // キーボード入力バッファ用の変数
    let kb_buffer = Arc::new(Mutex::new(String::new()));

    // イベント監視スレッド
    {
        let display_infos = Arc::clone(&display_infos);
        let is_recording = Arc::clone(&is_recording_for_event);
        let log_sender = log_sender_for_event;
        let image_counter = Arc::clone(&image_counter_for_event);
        let pending_saves = Arc::clone(&pending_saves_for_event);
        let current_session_folder = Arc::clone(&current_session_folder_for_event);
        let kb_buffer_clone = Arc::clone(&kb_buffer);

        thread::spawn(move || {
            // 現在のマウス位置で初期化
            let initial_pos = get_current_mouse_pos();
            let last_mouse_pos = Arc::new(AtomicU64::new(pack_mouse(initial_pos.0, initial_pos.1)));
            let last_mouse_pos_clone = Arc::clone(&last_mouse_pos);

            if let Err(e) = listen(move |event| {
                if let EventType::MouseMove { x, y } = event.event_type {
                    last_mouse_pos_clone.store(pack_mouse(x, y), Ordering::Relaxed);
                }

                if !is_recording.load(Ordering::Relaxed) {
                    return;
                }
                let mut pending_guard = PendingWorkGuard::new(&pending_saves);

                // キー入力イベントのバッファリング
                if let EventType::KeyPress(key) = event.event_type {
                    let is_modifier = matches!(
                        key,
                        Key::ControlLeft
                            | Key::ControlRight
                            | Key::ShiftLeft
                            | Key::ShiftRight
                            | Key::Alt
                            | Key::AltGr
                            | Key::MetaLeft
                            | Key::MetaRight
                    );

                    if !is_modifier {
                        if key == Key::Backspace {
                            let mut buf = kb_buffer_clone.lock().unwrap();
                            buf.pop();
                        } else if key != Key::Return && key != Key::Tab {
                            if let Some(ref name) = event.name {
                                if name.chars().all(|c| !c.is_control()) {
                                    let mut buf = kb_buffer_clone.lock().unwrap();
                                    buf.push_str(name);
                                }
                            }
                        }
                    }
                }

                let mut buffered_text = String::new();
                let should_flush = matches!(
                    event.event_type,
                    EventType::ButtonPress(Button::Left)
                        | EventType::ButtonPress(Button::Right)
                        | EventType::KeyPress(Key::Return)
                        | EventType::KeyPress(Key::Tab)
                );

                if should_flush {
                    let mut buf = kb_buffer_clone.lock().unwrap();
                    if !buf.is_empty() {
                        buffered_text = std::mem::take(&mut *buf);
                    }
                }

                let (action, needs_mouse_pos) = match event.event_type {
                    EventType::ButtonPress(Button::Left) => {
                        let act = if buffered_text.is_empty() {
                            "MouseClick".to_string()
                        } else {
                            format!("MouseClick (入力: 「{}」)", buffered_text)
                        };
                        (act, true)
                    }
                    EventType::ButtonPress(Button::Right) => {
                        let act = if buffered_text.is_empty() {
                            "RightClick".to_string()
                        } else {
                            format!("RightClick (入力: 「{}」)", buffered_text)
                        };
                        (act, true)
                    }
                    EventType::KeyPress(Key::Return) => {
                        if buffered_text.is_empty() {
                            ("KeyPress_Enter".to_string(), true)
                        } else {
                            (format!("KeyPress_Enter (入力: 「{}」)", buffered_text), true)
                        }
                    }
                    EventType::KeyPress(Key::Tab) => {
                        if buffered_text.is_empty() {
                            ("KeyPress_Tab".to_string(), true)
                        } else {
                            (format!("KeyPress_Tab (入力: 「{}」)", buffered_text), true)
                        }
                    }
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

                    let session_folder = current_session_folder
                        .lock()
                        .unwrap()
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("セッションフォルダが未設定: {}", action))?;

                    let mut counter = image_counter.lock().unwrap();
                    *counter += 1;
                    let image_index = *counter;
                    drop(counter);

                    let window_title = get_active_window_title();

                    let trigger = RecordTrigger {
                        target_display,
                        mouse_pos: mouse_pos.unwrap_or((0.0, 0.0)),
                        has_mouse_pos: mouse_pos.is_some(),
                        timestamp: timestamp.clone(),
                        action: action.clone(),
                        session_folder,
                        image_index,
                        window_title: Some(window_title.clone()),
                    };

                    // キューが一杯ならスキップ（OSのフックやメモリをフリーズさせないため）
                    bg_sender
                        .try_send(trigger)
                        .map_err(|_| anyhow::anyhow!("【警告】保存処理が追いついていません。このクリックはスキップされました: {}", action))?;
                    pending_guard.dismiss();

                    let display_action = format!("[{}] {}", window_title, action);
                    log_sender
                        .send(format!("[{}] {} を記録", timestamp, display_action))
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

    let log_sender_for_gui = log_sender.clone();

    eframe::run_native(
        "PC操作ロガー",
        options,
        Box::new(move |cc| {
            let app = RecorderApp::new(
                cc,
                is_recording_for_gui,
                log_receiver,
                log_sender_for_gui,
                image_counter_for_gui,
                pending_saves_for_gui,
                session_sender,
            );

            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI起動エラー: {}", e))?;

    Ok(())
}
