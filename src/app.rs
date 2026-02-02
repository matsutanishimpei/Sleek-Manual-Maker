use chrono::Local;
use eframe::egui;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use crate::generator;
use crate::types::{AppState, OperationLog};

#[cfg(debug_assertions)]
use screenshots::Screen;

// GUIアプリケーション構造体
pub struct RecorderApp {
    state: AppState,
    is_recording: Arc<AtomicBool>,
    log_receiver: mpsc::Receiver<String>,
    log_messages: Vec<String>,
    current_session_folder: Option<PathBuf>,
    review_logs: Vec<OperationLog>,
    image_counter: Arc<Mutex<usize>>,

    session_sender: mpsc::Sender<PathBuf>,
    #[cfg(debug_assertions)]
    debug_monitor_info: String,
}

impl RecorderApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        is_recording: Arc<AtomicBool>,
        log_receiver: mpsc::Receiver<String>,
        image_counter: Arc<Mutex<usize>>,
        session_sender: mpsc::Sender<PathBuf>,
    ) -> Self {
        // BIZ UDゴシックフォントをバイナリに埋め込み
        let mut fonts = egui::FontDefinitions::default();
        
        let biz_ud_gothic_data = include_bytes!("../assets/BIZUDPGothic-Regular.ttf");
        
        fonts.font_data.insert(
            "BIZ_UD_Gothic".to_owned(),
            egui::FontData::from_static(biz_ud_gothic_data),
        );
        
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "BIZ_UD_Gothic".to_owned());
        
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "BIZ_UD_Gothic".to_owned());
        
        cc.egui_ctx.set_fonts(fonts);
        
        // 画像ローダーをインストール
        egui_extras::install_image_loaders(&cc.egui_ctx);

        #[cfg(debug_assertions)]
        let debug_monitor_info = {
            match Screen::all() {
                Ok(screens) => {
                    let infos: Vec<String> = screens
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            format!(
                                "Monitor {}: {}x{}",
                                i + 1,
                                s.display_info.width,
                                s.display_info.height
                            )
                        })
                        .collect();
                    format!("{} Monitors [{}]", screens.len(), infos.join(", "))
                }
                Err(e) => format!("Failed to get monitor info: {}", e),
            }
        };
        
        Self {
            state: AppState::Idle,
            is_recording,
            log_receiver,
            log_messages: Vec::new(),
            current_session_folder: None,
            review_logs: Vec::new(),
            image_counter,
            session_sender,
            #[cfg(debug_assertions)]
            debug_monitor_info,
        }
    }
    
    fn start_recording(&mut self) {
        // セッションフォルダを作成
        let now = Local::now();
        let folder_name = format!("records/{}", now.format("%Y%m%d-%H%M%S"));
        let session_folder = PathBuf::from(&folder_name);
        
        if let Err(e) = fs::create_dir_all(&session_folder) {
            eprintln!("セッションフォルダの作成に失敗: {}", e);
            return;
        }
        
        self.current_session_folder = Some(session_folder.clone());
        *self.image_counter.lock().unwrap() = 0;
        self.log_messages.clear();
        
        // バックグラウンドスレッドにセッションフォルダを送信
        let _ = self.session_sender.send(session_folder);
        
        self.state = AppState::Recording;
        self.is_recording.store(true, Ordering::Relaxed);
    }
    
    fn stop_recording(&mut self) {
        self.is_recording.store(false, Ordering::Relaxed);
        self.state = AppState::Review;
        
        // レビュー用にログを読み込み
        if let Some(ref folder) = self.current_session_folder {
            let log_path = folder.join("session_log.jsonl");
            if let Ok(content) = fs::read_to_string(&log_path) {
                self.review_logs = content
                    .lines()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect();
            }
        }
    }
    
    fn delete_image(&mut self, index: usize) {
        if index >= self.review_logs.len() {
            return;
        }
        
        // ファイルを削除
        let log_entry = &self.review_logs[index];
        if let Some(ref folder) = self.current_session_folder {
            let image_path = folder.join(&log_entry.image_path);
            let _ = fs::remove_file(image_path);
        }
        
        // メモリから削除
        self.review_logs.remove(index);
        
        // JSONLを更新
        if let Some(ref folder) = self.current_session_folder {
            let log_path = folder.join("session_log.jsonl");
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_path)
            {
                for log in &self.review_logs {
                    if let Ok(json) = serde_json::to_string(log) {
                        let _ = writeln!(file, "{}", json);
                    }
                }
            }
        }
    }
    
    fn finish_review(&mut self) {
        // HTMLマニュアルを生成
        if let Some(ref folder) = self.current_session_folder {
            match generator::generate_html(folder, &self.review_logs) {
                Ok(path) => {
                    self.log_messages.push(format!("✅ HTMLマニュアルを生成しました: {:?}", path));
                    // ブラウザで自動的に開く
                    if let Err(e) = open::that(&path) {
                        eprintln!("ブラウザを開けませんでした: {}", e);
                        self.log_messages.push(format!("⚠️ ブラウザ起動エラー: {}", e));
                    }
                },
                Err(e) => {
                    eprintln!("HTML生成エラー: {}", e);
                    self.log_messages.push(format!("❌ HTML生成エラー: {}", e));
                }
            }
        }

        self.state = AppState::Idle;
        self.current_session_folder = None;
        self.review_logs.clear();
        self.log_messages.clear();
    }

    fn render_idle(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎥 PC操作ロガー");
            ui.add_space(20.0);
            
            ui.label(
                egui::RichText::new("待機中")
                    .size(20.0)
                    .color(egui::Color32::from_rgb(100, 100, 100))
            );
            ui.add_space(20.0);
            
            if ui.add(egui::Button::new("▶ 録画開始").min_size(egui::vec2(150.0, 50.0))).clicked() {
                self.start_recording();
            }
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            
            ui.label("💡 録画開始ボタンを押すと、マウスクリックと右クリックを記録します");
            ui.label("📁 記録は records/YYYYMMDD-HHMMSS/ フォルダに保存されます");
        });
    }
    
    fn render_recording(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎥 PC操作ロガー");
            ui.add_space(10.0);

            ui.label(
                egui::RichText::new("● 録画中")
                    .size(20.0)
                    .color(egui::Color32::from_rgb(255, 50, 50))
            );
            ui.add_space(10.0);

            if ui.add(egui::Button::new("⏹ 録画停止").min_size(egui::vec2(120.0, 40.0))).clicked() {
                self.stop_recording();
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label("📋 ログ:");
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &self.log_messages {
                        ui.label(msg);
                    }
                });
        });
    }
    
    fn render_review(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📸 録画内容の確認");
            ui.add_space(10.0);
            
            if let Some(ref folder) = self.current_session_folder {
                ui.label(format!("📁 保存先: {}", folder.display()));
            }
            ui.label(format!("📊 記録数: {}件", self.review_logs.len()));
            ui.add_space(10.0);
            
            if ui.add(egui::Button::new("✅ 保存して終了").min_size(egui::vec2(150.0, 40.0))).clicked() {
                self.finish_review();
            }
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            // 画像グリッド表示
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])  // 自動縮小を無効化
                .show(ui, |ui| {
                let mut to_delete = None;
                
                for (index, log) in self.review_logs.iter().enumerate() {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            // ヘッダー情報
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("#{}", index + 1)).size(16.0).strong());
                                ui.label(&log.action);
                                if let (Some(x), Some(y)) = (log.x, log.y) {
                                    ui.label(egui::RichText::new(format!("座標: ({}, {})", x, y)).color(egui::Color32::from_rgb(100, 100, 255)));
                                }
                            });
                            
                            ui.add_space(10.0);
                            
                            // 画像表示（3倍サイズ）
                            if let Some(ref folder) = self.current_session_folder {
                                let image_path = folder.join(&log.image_path);
                                if image_path.exists() {
                                    let uri = format!("file://{}", image_path.display());
                                    
                                    // 画像を大きく表示（900x600の範囲内で縦横比を保持）
                                    let target_size = egui::vec2(900.0, 600.0);
                                    let response = ui.add(
                                        egui::Image::new(&uri)
                                            .fit_to_exact_size(target_size)
                                            .maintain_aspect_ratio(true)
                                    );
                                    
                                    // クリック位置に赤い円を描画
                                    if let (Some(x), Some(y)) = (log.x, log.y) {
                                        let rect = response.rect;
                                        
                                        // 画像の実際のサイズを取得
                                        if let Ok(img) = image::open(&image_path) {
                                            let img_w = img.width() as f32;
                                            let img_h = img.height() as f32;
                                            
                                            // 実際に描画されている領域を計算（object-fit: contain相当）
                                            let img_aspect = img_w / img_h;
                                            let rect_aspect = rect.width() / rect.height();
                                            
                                            let (display_w, display_h) = if img_aspect > rect_aspect {
                                                // 横長：横幅に合わせて縦に余白
                                                (rect.width(), rect.width() / img_aspect)
                                            } else {
                                                // 縦長：縦幅に合わせて横に余白
                                                (rect.height() * img_aspect, rect.height())
                                            };
                                            
                                            // 画像の描画開始位置（中央寄せ）
                                            let offset_x = rect.left() + (rect.width() - display_w) / 2.0;
                                            let offset_y = rect.top() + (rect.height() - display_h) / 2.0;

                                            // クリック位置を画面座標に変換
                                            let scale = display_w / img_w;
                                            let click_x = offset_x + (x as f32 * scale);
                                            let click_y = offset_y + (y as f32 * scale);
                                            
                                            let center = egui::pos2(click_x, click_y);
                                            
                                            // 大きな赤い円を描画（半径30px）
                                            ui.painter().circle_stroke(
                                                center,
                                                30.0,
                                                egui::Stroke::new(4.0, egui::Color32::from_rgb(255, 0, 0))
                                            );
                                            
                                            // 中心点
                                            ui.painter().circle_filled(
                                                center,
                                                6.0,
                                                egui::Color32::from_rgb(255, 0, 0)
                                            );
                                        }
                                    }
                                }
                            }
                            
                            ui.add_space(10.0);
                            
                            // 削除ボタン
                            if ui.add(egui::Button::new("🗑️ 削除").min_size(egui::vec2(100.0, 30.0))).clicked() {
                                to_delete = Some(index);
                            }
                        });
                    });
                    
                    ui.add_space(15.0);
                }
                
                // 削除処理（イテレーション外で実行）
                if let Some(index) = to_delete {
                    self.delete_image(index);
                }
            });
        });
    }
}

impl eframe::App for RecorderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // チャネルから新しいログメッセージを受信
        let mut new_messages = Vec::new();
        while let Ok(msg) = self.log_receiver.try_recv() {
            new_messages.push(msg);
        }

        if !new_messages.is_empty() {
             // ログディレクトリの作成
             fs::create_dir_all("log").ok();

             // アプリケーションログファイルに追記
             if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("log/application.log") 
            {
                for msg in &new_messages {
                    let _ = writeln!(file, "{}", msg);
                }
            }

            // GUIバッファに追加
            for msg in new_messages {
                self.log_messages.push(msg);
                if self.log_messages.len() > 100 {
                    self.log_messages.remove(0);
                }
            }
        }



        #[cfg(debug_assertions)]
        egui::TopBottomPanel::bottom("debug_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("🐛 Debug Info:")
                        .strong()
                        .color(egui::Color32::YELLOW),
                );
                ui.label(&self.debug_monitor_info);
            });
        });

        match self.state {
            AppState::Idle => self.render_idle(ctx),
            AppState::Recording => self.render_recording(ctx),
            AppState::Review => self.render_review(ctx),
        }

        ctx.request_repaint();
    }
}
