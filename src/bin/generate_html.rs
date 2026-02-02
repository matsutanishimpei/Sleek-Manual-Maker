use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Local};
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

// JSONLログエントリの構造体
#[derive(Deserialize, Debug)]
struct OperationLog {
    timestamp: String,
    action: String,
    x: Option<i32>,
    y: Option<i32>,
    target_monitor_id: u32,
    image_path: String,
}

/// 画像ファイルをBase64エンコードしてdata URIを生成
fn image_to_base64_data_uri(image_path: &str) -> String {
    match std::fs::read(image_path) {
        Ok(image_data) => {
            let base64_string = general_purpose::STANDARD.encode(&image_data);
            format!("data:image/png;base64,{}", base64_string)
        }
        Err(e) => {
            eprintln!("⚠️  警告: 画像ファイルの読み込みに失敗しました: {}", image_path);
            eprintln!("   エラー: {}", e);
            format!("[Image not found: {}]", image_path)
        }
    }
}

fn main() -> Result<()> {
    println!("📝 HTML手順書生成ツール (Base64埋め込み版)");
    println!("=====================================");

    // session_log.jsonlを読み込み
    let input_file = "session_log.jsonl";
    let output_file = "manual.html";

    println!("📖 読み込み中: {}", input_file);

    let file = File::open(input_file)?;
    let reader = BufReader::new(file);

    // JSONLを1行ずつパース
    let mut operations: Vec<OperationLog> = Vec::new();
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<OperationLog>(&line) {
            Ok(op) => operations.push(op),
            Err(e) => {
                eprintln!("⚠️  警告: {}行目のパースに失敗しました: {}", line_num + 1, e);
                eprintln!("   内容: {}", line);
            }
        }
    }

    println!("✅ {}件の操作を読み込みました", operations.len());

    if operations.is_empty() {
        println!("⚠️  操作ログが空です。手順書を生成できません。");
        return Ok(());
    }

    // HTML生成
    println!("📝 HTML生成中: {}", output_file);
    println!("🖼️  画像をBase64エンコード中...");

    let mut html_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)?;

    // HTMLヘッダー
    let now = Local::now();
    writeln!(html_file, "<!DOCTYPE html>")?;
    writeln!(html_file, "<html lang=\"ja\">")?;
    writeln!(html_file, "<head>")?;
    writeln!(html_file, "    <meta charset=\"UTF-8\">")?;
    writeln!(html_file, "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(html_file, "    <title>操作手順書</title>")?;
    writeln!(html_file, "    <style>")?;
    writeln!(html_file, "        * {{")?;
    writeln!(html_file, "            margin: 0;")?;
    writeln!(html_file, "            padding: 0;")?;
    writeln!(html_file, "            box-sizing: border-box;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        body {{")?;
    writeln!(html_file, "            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;")?;
    writeln!(html_file, "            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);")?;
    writeln!(html_file, "            min-height: 100vh;")?;
    writeln!(html_file, "            padding: 40px 20px;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .container {{")?;
    writeln!(html_file, "            max-width: 900px;")?;
    writeln!(html_file, "            margin: 0 auto;")?;
    writeln!(html_file, "            background: white;")?;
    writeln!(html_file, "            border-radius: 20px;")?;
    writeln!(html_file, "            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);")?;
    writeln!(html_file, "            overflow: hidden;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .header {{")?;
    writeln!(html_file, "            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);")?;
    writeln!(html_file, "            color: white;")?;
    writeln!(html_file, "            padding: 40px;")?;
    writeln!(html_file, "            text-align: center;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .header h1 {{")?;
    writeln!(html_file, "            font-size: 2.5em;")?;
    writeln!(html_file, "            margin-bottom: 15px;")?;
    writeln!(html_file, "            font-weight: 700;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .header-info {{")?;
    writeln!(html_file, "            font-size: 1.1em;")?;
    writeln!(html_file, "            opacity: 0.95;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .content {{")?;
    writeln!(html_file, "            padding: 40px;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-card {{")?;
    writeln!(html_file, "            background: #f8f9fa;")?;
    writeln!(html_file, "            border-radius: 15px;")?;
    writeln!(html_file, "            padding: 30px;")?;
    writeln!(html_file, "            margin-bottom: 30px;")?;
    writeln!(html_file, "            box-shadow: 0 4px 15px rgba(0, 0, 0, 0.08);")?;
    writeln!(html_file, "            transition: transform 0.3s ease, box-shadow 0.3s ease;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-card:hover {{")?;
    writeln!(html_file, "            transform: translateY(-5px);")?;
    writeln!(html_file, "            box-shadow: 0 8px 25px rgba(0, 0, 0, 0.15);")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-header {{")?;
    writeln!(html_file, "            display: flex;")?;
    writeln!(html_file, "            align-items: center;")?;
    writeln!(html_file, "            margin-bottom: 20px;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-number {{")?;
    writeln!(html_file, "            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);")?;
    writeln!(html_file, "            color: white;")?;
    writeln!(html_file, "            width: 50px;")?;
    writeln!(html_file, "            height: 50px;")?;
    writeln!(html_file, "            border-radius: 50%;")?;
    writeln!(html_file, "            display: flex;")?;
    writeln!(html_file, "            align-items: center;")?;
    writeln!(html_file, "            justify-content: center;")?;
    writeln!(html_file, "            font-size: 1.5em;")?;
    writeln!(html_file, "            font-weight: bold;")?;
    writeln!(html_file, "            margin-right: 20px;")?;
    writeln!(html_file, "            flex-shrink: 0;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-title {{")?;
    writeln!(html_file, "            font-size: 1.8em;")?;
    writeln!(html_file, "            color: #2d3748;")?;
    writeln!(html_file, "            font-weight: 600;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .step-info {{")?;
    writeln!(html_file, "            background: white;")?;
    writeln!(html_file, "            border-radius: 10px;")?;
    writeln!(html_file, "            padding: 20px;")?;
    writeln!(html_file, "            margin-bottom: 20px;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .info-row {{")?;
    writeln!(html_file, "            display: flex;")?;
    writeln!(html_file, "            margin-bottom: 10px;")?;
    writeln!(html_file, "            font-size: 1.05em;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .info-row:last-child {{")?;
    writeln!(html_file, "            margin-bottom: 0;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .info-label {{")?;
    writeln!(html_file, "            font-weight: 600;")?;
    writeln!(html_file, "            color: #667eea;")?;
    writeln!(html_file, "            min-width: 120px;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .info-value {{")?;
    writeln!(html_file, "            color: #4a5568;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .screenshot {{")?;
    writeln!(html_file, "            width: 100%;")?;
    writeln!(html_file, "            border-radius: 10px;")?;
    writeln!(html_file, "            box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);")?;
    writeln!(html_file, "            transition: transform 0.3s ease;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .screenshot:hover {{")?;
    writeln!(html_file, "            transform: scale(1.02);")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .footer {{")?;
    writeln!(html_file, "            background: #f8f9fa;")?;
    writeln!(html_file, "            padding: 30px;")?;
    writeln!(html_file, "            text-align: center;")?;
    writeln!(html_file, "            color: #718096;")?;
    writeln!(html_file, "            font-size: 1.1em;")?;
    writeln!(html_file, "            border-top: 3px solid #667eea;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .footer strong {{")?;
    writeln!(html_file, "            color: #2d3748;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "        .error-message {{")?;
    writeln!(html_file, "            background: #fed7d7;")?;
    writeln!(html_file, "            color: #c53030;")?;
    writeln!(html_file, "            padding: 15px;")?;
    writeln!(html_file, "            border-radius: 8px;")?;
    writeln!(html_file, "            border-left: 4px solid #c53030;")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "    </style>")?;
    writeln!(html_file, "</head>")?;
    writeln!(html_file, "<body>")?;
    writeln!(html_file, "    <div class=\"container\">")?;
    writeln!(html_file, "        <div class=\"header\">")?;
    writeln!(html_file, "            <h1>📋 操作手順書</h1>")?;
    writeln!(
        html_file,
        "            <div class=\"header-info\">生成日時: {} | 操作数: {}件</div>",
        now.format("%Y年%m月%d日 %H:%M:%S"),
        operations.len()
    )?;
    writeln!(html_file, "        </div>")?;
    writeln!(html_file, "        <div class=\"content\">")?;

    // 各操作をステップとして出力
    for (i, op) in operations.iter().enumerate() {
        let step_num = i + 1;

        writeln!(html_file, "            <div class=\"step-card\">")?;
        writeln!(html_file, "                <div class=\"step-header\">")?;
        writeln!(html_file, "                    <div class=\"step-number\">{}</div>", step_num)?;
        writeln!(html_file, "                    <div class=\"step-title\">Step {}</div>", step_num)?;
        writeln!(html_file, "                </div>")?;

        // タイムスタンプをパースして整形
        let timestamp_str = match DateTime::parse_from_rfc3339(&op.timestamp) {
            Ok(dt) => dt.format("%Y年%m月%d日 %H:%M:%S").to_string(),
            Err(_) => op.timestamp.clone(),
        };

        // アクション名を日本語化
        let action_jp = match op.action.as_str() {
            "MouseClick" => "マウスクリック",
            "KeyPress_Tab" => "Tabキー押下",
            _ => &op.action,
        };

        writeln!(html_file, "                <div class=\"step-info\">")?;
        writeln!(html_file, "                    <div class=\"info-row\">")?;
        writeln!(html_file, "                        <div class=\"info-label\">📅 日時:</div>")?;
        writeln!(html_file, "                        <div class=\"info-value\">{}</div>", timestamp_str)?;
        writeln!(html_file, "                    </div>")?;
        writeln!(html_file, "                    <div class=\"info-row\">")?;
        writeln!(html_file, "                        <div class=\"info-label\">🎯 アクション:</div>")?;
        writeln!(
            html_file,
            "                        <div class=\"info-value\">{} (Monitor: {})</div>",
            action_jp, op.target_monitor_id
        )?;
        writeln!(html_file, "                    </div>")?;

        // マウス座標（ある場合のみ）
        if let (Some(x), Some(y)) = (op.x, op.y) {
            writeln!(html_file, "                    <div class=\"info-row\">")?;
            writeln!(html_file, "                        <div class=\"info-label\">📍 座標:</div>")?;
            writeln!(html_file, "                        <div class=\"info-value\">({}, {})</div>", x, y)?;
            writeln!(html_file, "                    </div>")?;
        }

        writeln!(html_file, "                </div>")?;

        // スクリーンショット画像をBase64エンコードして埋め込み
        print!("  Step {}: 画像を処理中... ", step_num);
        let data_uri = image_to_base64_data_uri(&op.image_path);

        if data_uri.starts_with("data:image") {
            println!("✅");
            writeln!(
                html_file,
                "                <img src=\"{}\" alt=\"スクリーンショット\" class=\"screenshot\">",
                data_uri
            )?;
        } else {
            println!("❌");
            writeln!(html_file, "                <div class=\"error-message\">{}</div>", data_uri)?;
        }

        writeln!(html_file, "            </div>")?;
    }

    // フッター
    writeln!(html_file, "        </div>")?;
    writeln!(html_file, "        <div class=\"footer\">")?;
    writeln!(
        html_file,
        "            <strong>✅ 完了</strong> - 以上、{}ステップの操作手順でした。",
        operations.len()
    )?;
    writeln!(html_file, "        </div>")?;
    writeln!(html_file, "    </div>")?;
    writeln!(html_file, "</body>")?;
    writeln!(html_file, "</html>")?;

    println!("✅ 手順書を生成しました: {}", output_file);
    println!("   ステップ数: {}", operations.len());
    println!("   📌 画像はBase64形式で埋め込まれています");
    println!("   💡 ブラウザで開いて確認してください");

    Ok(())
}
