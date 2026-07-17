use chrono::Local;
use eframe::egui;
use std::collections::VecDeque;
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
    log_sender: mpsc::Sender<String>,
    log_messages: VecDeque<String>,
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
        log_sender: mpsc::Sender<String>,
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

        // モダンな和風ダークスタイル（墨色ベース、低コントラストでぎらつかない）
        let mut visuals = egui::Visuals::dark();
        
        let sumi_black = egui::Color32::from_rgb(30, 31, 33);     // 墨色（ベース背景）
        let shikkoku = egui::Color32::from_rgb(22, 23, 24);       // 漆黒（より深い背景）
        let kinari = egui::Color32::from_rgb(220, 216, 206);      // 生成り色（柔らかい文字色）
        let matcha = egui::Color32::from_rgb(110, 133, 101);      // 抹茶色（プライマリアクセント）
        let soft_gray = egui::Color32::from_rgb(60, 62, 66);      // ソフトグレー（枠線・ウィジェット背景）
        let soft_gray_hover = egui::Color32::from_rgb(76, 78, 84);
        
        visuals.window_fill = sumi_black;
        visuals.panel_fill = sumi_black;
        visuals.extreme_bg_fill = shikkoku;
        visuals.override_text_color = Some(kinari);
        
        visuals.widgets.noninteractive.bg_fill = sumi_black;
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, soft_gray);
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, kinari);
        
        visuals.widgets.inactive.bg_fill = soft_gray;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, soft_gray);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, kinari);
        visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        
        visuals.widgets.hovered.bg_fill = soft_gray_hover;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, matcha);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, kinari);
        visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        
        visuals.widgets.active.bg_fill = matcha;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, matcha);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, shikkoku);
        visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        
        visuals.widgets.open.bg_fill = soft_gray;
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, soft_gray);
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, kinari);
        
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.window_stroke = egui::Stroke::new(1.0, soft_gray);
        
        cc.egui_ctx.set_visuals(visuals);

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
            log_sender,
            log_messages: VecDeque::new(),
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
    
    fn get_session_size_mb(&self) -> f32 {
        let mut total_bytes = 0u64;
        if let Some(ref folder) = self.current_session_folder {
            if let Ok(entries) = fs::read_dir(folder) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            total_bytes += metadata.len();
                        }
                    }
                }
            }
        }
        (total_bytes as f32) / (1024.0 * 1024.0)
    }
    
    fn finish_review(&mut self) {
        // HTMLマニュアルを生成（時間のかかるBase64エンコード処理等をバックグラウンドスレッドへ非同期にオフロード）
        if let Some(ref folder) = self.current_session_folder {
            let folder = folder.clone();
            let logs = self.review_logs.clone();
            let log_sender = self.log_sender.clone();
            
            self.log_messages.push_back("⏳ HTML手順書をバックグラウンドで作成中...".to_string());
            
            std::thread::spawn(move || {
                match generator::generate_html(&folder, &logs) {
                    Ok(path) => {
                        let _ = log_sender.send(format!("✅ HTMLマニュアルを生成しました: {:?}", path));
                        // ブラウザで自動的に開く
                        if let Err(e) = open::that(&path) {
                            eprintln!("ブラウザを開けませんでした: {}", e);
                            let _ = log_sender.send(format!("⚠ ブラウザ起動エラー: {}", e));
                        }
                    },
                    Err(e) => {
                        eprintln!("HTML生成エラー: {}", e);
                        let _ = log_sender.send(format!("❌ HTML生成エラー: {}", e));
                    }
                }
            });
        }

        self.state = AppState::Idle;
        self.current_session_folder = None;
        self.review_logs.clear();
        // log_messages はここでクリアしない
        // → 「✅ HTML生成完了」等のメッセージを Idle 画面でも確認できる
        // → 次の録画開始時に start_recording() の clear() で消去される
    }

    fn cancel_recording(&mut self) {
        // 録画イベントの監視フラグを確実に停止する
        self.is_recording.store(false, Ordering::Relaxed);

        if let Some(ref folder) = self.current_session_folder {
            // セッションフォルダを削除してデータを破棄（エラーは無視）
            let _ = std::fs::remove_dir_all(folder);
        }
        
        self.state = AppState::Idle;
        self.current_session_folder = None;
        self.review_logs.clear();
        self.log_messages.clear();
        self.log_messages.push_back("⚠ 録画をキャンセルし、データを破棄しました。".to_string());
    }

    fn render_idle(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🎥 PC操作ロガー").strong());
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(130, 130, 130)));
            });
            ui.add_space(20.0);

            ui.label(
                egui::RichText::new("待機中")
                    .size(18.0)
                    .color(egui::Color32::from_rgb(130, 130, 130))
            );
            ui.add_space(20.0);

            let start_btn = egui::Button::new(egui::RichText::new("▶ 録画開始").color(egui::Color32::from_rgb(22, 23, 24)).strong())
                .fill(egui::Color32::from_rgb(110, 133, 101))
                .min_size(egui::vec2(150.0, 50.0));

            if ui.add(start_btn).clicked() {
                self.start_recording();
            }

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            ui.label("💡 録画開始ボタンを押すと、マウスクリックと右クリックを記録します");
            ui.label("📁 記録は records/YYYYMMDD-HHMMSS/ フォルダに保存されます");

            // 前回セッションのログメッセージを最新5件表示
            if !self.log_messages.is_empty() {
                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("📋 前回セッションのログ:")
                        .color(egui::Color32::from_rgb(150, 150, 150))
                );
                ui.add_space(5.0);
                let recent: Vec<&String> = self.log_messages.iter().rev().take(5).collect();
                for msg in recent.into_iter().rev() {
                    ui.label(
                        egui::RichText::new(msg)
                            .size(13.0)
                            .color(egui::Color32::from_rgb(130, 130, 130))
                    );
                }
            }
        });
    }
    
    fn render_recording(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🎥 PC操作ロガー").strong());
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(130, 130, 130)));
            });
            ui.add_space(10.0);

            ui.label(
                egui::RichText::new("● 録画中")
                    .size(18.0)
                    .color(egui::Color32::from_rgb(176, 73, 73))
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let stop_btn = egui::Button::new(egui::RichText::new("⏹ 録画停止").color(egui::Color32::from_rgb(22, 23, 24)).strong())
                    .fill(egui::Color32::from_rgb(93, 116, 141))
                    .min_size(egui::vec2(120.0, 40.0));
                if ui.add(stop_btn).clicked() {
                    self.stop_recording();
                }
                
                ui.add_space(20.0);
                
                let cancel_btn = egui::Button::new(egui::RichText::new("🗑 録画をキャンセル").color(egui::Color32::from_rgb(242, 240, 235)).strong())
                    .fill(egui::Color32::from_rgb(166, 68, 68))
                    .min_size(egui::vec2(150.0, 40.0));
                if ui.add(cancel_btn).clicked() {
                    self.cancel_recording();
                }
            });

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
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("📸 録画セッションの完了").strong());
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(130, 130, 130)));
            });
            ui.add_space(15.0);

            let session_size = self.get_session_size_mb();
            
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("📋 セッション情報のサマリー").size(16.0).strong());
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📊 記録ステップ数:").strong());
                        ui.label(format!("{} 件", self.review_logs.len()));
                    });
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("💾 記録データ容量:").strong());
                        ui.label(format!("{:.2} MB", session_size));
                    });
                    ui.add_space(5.0);

                    if let Some(ref folder) = self.current_session_folder {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("📁 セッション保存先:").strong());
                            ui.label(folder.display().to_string());
                        });
                    }
                });
            });
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(egui::RichText::new("✅ HTML手順書を生成してブラウザで開く").color(egui::Color32::from_rgb(22, 23, 24)).strong())
                    .fill(egui::Color32::from_rgb(110, 133, 101))
                    .min_size(egui::vec2(250.0, 45.0));
                if ui.add(save_btn).clicked() {
                    self.finish_review();
                }
                
                ui.add_space(15.0);
                
                let discard_btn = egui::Button::new(egui::RichText::new("🗑 キャンセルして破棄").color(egui::Color32::from_rgb(242, 240, 235)).strong())
                    .fill(egui::Color32::from_rgb(166, 68, 68))
                    .min_size(egui::vec2(180.0, 45.0));
                if ui.add(discard_btn).clicked() {
                    self.cancel_recording();
                }
            });

            ui.add_space(25.0);
            ui.separator();
            ui.add_space(15.0);

            ui.vertical(|ui| {
                ui.label(egui::RichText::new("💡 各ステップの編集について").color(egui::Color32::from_rgb(150, 150, 150)).strong());
                ui.add_space(5.0);
                ui.label(egui::RichText::new("・不要なステップの削除や並び替え").color(egui::Color32::from_rgb(130, 130, 130)));
                ui.label(egui::RichText::new("・赤点マーカー（クリック座標）のON/OFF切り替え").color(egui::Color32::from_rgb(130, 130, 130)));
                ui.label(egui::RichText::new("・個人情報などを隠す「黒塗りマスク」の追加").color(egui::Color32::from_rgb(130, 130, 130)));
                ui.label(egui::RichText::new("・各ステップの説明文の直接入力").color(egui::Color32::from_rgb(130, 130, 130)));
                ui.add_space(5.0);
                ui.label(egui::RichText::new("これらは全て、生成されたHTML上で直感的に行うことができます。").color(egui::Color32::from_rgb(130, 130, 130)).italics());
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
                self.log_messages.push_back(msg);
                if self.log_messages.len() > 100 {
                    self.log_messages.pop_front();
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

        // 録画中はリアルタイム更新、待機・レビュー中は低頻度で十分
        match self.state {
            AppState::Recording => ctx.request_repaint(),
            _ => ctx.request_repaint_after(std::time::Duration::from_millis(200)),
        }
    }
}
