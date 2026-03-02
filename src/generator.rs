use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::types::OperationLog;

// ─────────────────────────────────────────────
// CSS / JS 定数（writeln! のエスケープ地獄を廃止）
// ─────────────────────────────────────────────

const HTML_CSS: &str = r#"
        :root { --primary: #667eea; --danger: #e53e3e; --text: #2d3748; --bg: #f7fafc; }
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); padding: 40px 20px; line-height: 1.6; }
        .container { max-width: 1000px; margin: 0 auto; }
        .header { text-align: center; margin-bottom: 40px; }
        .header h1 { font-size: 2.5rem; color: #4a5568; margin-bottom: 10px; }
        .meta-info { color: #718096; font-size: 0.9rem; }
        .step-card { background: white; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.05); margin-bottom: 30px; overflow: hidden; transition: transform 0.2s; border: 1px solid #edf2f7; }
        .step-card:hover { box-shadow: 0 10px 15px rgba(0,0,0,0.1); }
        .card-header { background: #f8fafc; padding: 15px 25px; border-bottom: 1px solid #edf2f7; display: flex; justify-content: space-between; align-items: center; }
        .step-badge { background: var(--primary); color: white; padding: 4px 12px; border-radius: 20px; font-weight: bold; font-size: 0.9rem; }
        .controls { display: flex; gap: 10px; }
        .btn { padding: 6px 12px; border-radius: 6px; border: 1px solid #cbd5e0; background: white; cursor: pointer; font-size: 0.85rem; transition: all 0.2s; }
        .btn:hover { background: #edf2f7; }
        .btn-toggle.active { background: #ebf8ff; border-color: #4299e1; color: #2b6cb0; }
        .btn-delete { color: var(--danger); border-color: #feb2b2; }
        .btn-delete:hover { background: #fff5f5; }
        .card-body { display: flex; flex-wrap: wrap; }
        .image-area { flex: 2; min-width: 300px; padding: 20px; border-right: 1px solid #edf2f7; position: relative; }
        .image-container { position: relative; width: 100%; border-radius: 6px; overflow: hidden; border: 1px solid #e2e8f0; line-height: 0; }
        .screenshot { width: 100%; height: auto; display: block; }
        @keyframes ripple { 0% { transform: translate(-50%, -50%) scale(1); opacity: 1; } 100% { transform: translate(-50%, -50%) scale(3); opacity: 0; } }
        .marker { position: absolute; width: 30px; height: 30px; border: 3px solid rgba(255, 0, 0, 0.8); background-color: rgba(255, 0, 0, 0.2); border-radius: 50%; transform: translate(-50%, -50%); pointer-events: none; z-index: 10; box-shadow: 0 0 10px rgba(255,0,0,0.5); }
        .marker::before { content: ''; position: absolute; top: 50%; left: 50%; width: 100%; height: 100%; border: 2px solid rgba(255, 0, 0, 0.8); border-radius: 50%; animation: ripple 1.5s infinite ease-out; box-sizing: border-box; }
        .marker::after { content: ''; position: absolute; top: 50%; left: 50%; width: 6px; height: 6px; background-color: red; border-radius: 50%; transform: translate(-50%, -50%); }
        .desc-area { flex: 1; min-width: 250px; padding: 25px; display: flex; flex-direction: column; }
        .action-title { font-size: 1.1rem; font-weight: 600; color: #2d3748; margin-bottom: 15px; border-bottom: 2px solid #edf2f7; padding-bottom: 10px; }
        .description-box { flex-grow: 1; padding: 15px; border: 1px solid #e2e8f0; border-radius: 6px; background: #ffffb020; min-height: 100px; outline: none; transition: border-color 0.2s; color: #4a5568; white-space: pre-wrap; }
        .description-box:focus { border-color: var(--primary); background: white; box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1); }
        .description-box:empty:before { content: 'ここに手順の説明を入力...'; color: #a0aec0; }
        .save-bar { position: fixed; bottom: 20px; right: 20px; z-index: 100; }
        .btn-save { background: var(--primary); color: white; padding: 15px 30px; border-radius: 50px; border: none; font-weight: bold; font-size: 1.1rem; box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4); cursor: pointer; transition: transform 0.2s; }
        .btn-save:hover { transform: translateY(-2px); box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5); }
        .btn-save:active { transform: translateY(0); }
        @media (max-width: 768px) { .card-body { flex-direction: column; } .image-area { border-right: none; border-bottom: 1px solid #edf2f7; } }
"#;

const HTML_JS: &str = r#"
        function toggleDot(stepId, btn) {
            const step = document.getElementById(stepId);
            const marker = step.querySelector('.marker');
            if (marker) {
                const isHidden = marker.style.display === 'none';
                marker.style.display = isHidden ? 'block' : 'none';
                btn.classList.toggle('active', isHidden);
                btn.innerText = isHidden ? '🔴 赤点' : '⚪ 赤点なし';
            }
        }

        function deleteStep(stepId) {
            if(confirm('このステップを削除してもよろしいですか？')) {
                const el = document.getElementById(stepId);
                if(el) el.remove();
                // 残りのステップを1から振り直す
                const cards = document.querySelectorAll('.step-card');
                cards.forEach(function(card, index) {
                    const badge = card.querySelector('.step-badge');
                    if(badge) badge.textContent = 'STEP ' + (index + 1);
                });
            }
        }

        function saveHtml() {
            const htmlContent = '<!DOCTYPE html>\n' + document.documentElement.outerHTML;
            const blob = new Blob([htmlContent], { type: 'text/html' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'manual_edited.html';
            a.click();
        }
"#;

// ─────────────────────────────────────────────
// ヘルパー
// ─────────────────────────────────────────────

/// 画像ファイルをBase64エンコードしてdata URIを生成
fn image_to_base64_data_uri(image_path: &PathBuf) -> String {
    match std::fs::read(image_path) {
        Ok(image_data) => {
            format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(&image_data))
        }
        Err(e) => {
            eprintln!("警告: 画像ファイルの読み込みに失敗しました: {:?}", image_path);
            format!("[Image error: {}]", e)
        }
    }
}

/// 1ステップ分のカードHTMLを書き出す
fn write_step_card(f: &mut impl Write, session_folder: &PathBuf, log: &OperationLog, step_num: usize) -> Result<()> {
    let step_id = format!("step-{}", step_num);

    // ── カードヘッダー ──
    writeln!(f, "        <div class=\"step-card\" id=\"{}\">", step_id)?;
    writeln!(f, "            <div class=\"card-header\">")?;
    writeln!(f, "                <div class=\"step-badge\">STEP {}</div>", step_num)?;
    writeln!(f, "                <div class=\"controls\">")?;
    writeln!(f, "                    <button class=\"btn btn-toggle active\" onclick=\"toggleDot('{}', this)\">🔴 赤点</button>", step_id)?;
    writeln!(f, "                    <button class=\"btn btn-delete\" onclick=\"deleteStep('{}')\">🗑️ 削除</button>", step_id)?;
    writeln!(f, "                </div>")?;
    writeln!(f, "            </div>")?;

    // ── カードボディ ──
    writeln!(f, "            <div class=\"card-body\">")?;

    // 画像エリア
    writeln!(f, "                <div class=\"image-area\">")?;
    let image_path = session_folder.join(&log.image_path);
    if image_path.exists() {
        let data_uri = image_to_base64_data_uri(&image_path);
        writeln!(f, "                    <div class=\"image-container\">")?;
        writeln!(f, "                        <img src=\"{}\" class=\"screenshot\">", data_uri)?;

        // クリックマーカー
        if let (Some(x), Some(y)) = (log.x, log.y) {
            // log.width/height（物理px）を優先し、なければ image_dimensions で取得
            let (dim_w, dim_h) = match (log.width, log.height) {
                (Some(w), Some(h)) if w > 0 && h > 0 => (w as f64, h as f64),
                _ => {
                    let (w, h) = image::image_dimensions(&image_path).unwrap_or((0, 0));
                    (w as f64, h as f64)
                }
            };
            if dim_w > 0.0 {
                let left = ((x as f64 / dim_w) * 100.0).clamp(0.0, 100.0);
                let top  = ((y as f64 / dim_h) * 100.0).clamp(0.0, 100.0);
                writeln!(f, "                        <div class=\"marker\" style=\"left: {:.2}%; top: {:.2}%;\"></div>", left, top)?;
            }
        }

        writeln!(f, "                    </div>")?;
    } else {
        writeln!(f, "                    <div style=\"color:red; padding: 20px;\">画像が見つかりません</div>")?;
    }
    writeln!(f, "                </div>")?;

    // 説明エリア
    writeln!(f, "                <div class=\"desc-area\">")?;
    writeln!(f, "                    <div class=\"action-title\">{}</div>", log.action)?;
    writeln!(f, "                    <div class=\"description-box\" contenteditable=\"true\"></div>")?;
    writeln!(f, "                </div>")?;

    writeln!(f, "            </div>")?; // card-body
    writeln!(f, "        </div>")?;     // step-card
    Ok(())
}

// ─────────────────────────────────────────────
// エントリーポイント
// ─────────────────────────────────────────────

pub fn generate_html(session_folder: &PathBuf, logs: &[OperationLog]) -> Result<PathBuf> {
    let output_file = session_folder.join("manual.html");
    let mut f = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(&output_file)?;

    let now = Local::now();

    // ── <head> ──
    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html lang=\"ja\">")?;
    writeln!(f, "<head>")?;
    writeln!(f, "    <meta charset=\"UTF-8\">")?;
    writeln!(f, "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(f, "    <title>操作手順書</title>")?;
    writeln!(f, "    <style>{}</style>", HTML_CSS)?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;

    // ── ヘッダー ──
    writeln!(f, "    <div class=\"container\">")?;
    writeln!(f, "        <div class=\"header\">")?;
    writeln!(f, "            <h1>操作手順書</h1>")?;
    writeln!(f, "            <p class=\"meta-info\">生成日時: {} | ステップ数: {}</p>",
        now.format("%Y/%m/%d %H:%M"), logs.len())?;
    writeln!(f, "        </div>")?;

    // ── ステップカード ──
    for (i, log) in logs.iter().enumerate() {
        write_step_card(&mut f, session_folder, log, i + 1)?;
    }

    writeln!(f, "    </div>")?;

    // ── 保存ボタン ──
    writeln!(f, "    <div class=\"save-bar\">")?;
    writeln!(f, "        <button class=\"btn-save\" onclick=\"saveHtml()\">💾 手順書を保存</button>")?;
    writeln!(f, "    </div>")?;

    // ── スクリプト ──
    writeln!(f, "    <script>{}</script>", HTML_JS)?;

    writeln!(f, "</body>")?;
    writeln!(f, "</html>")?;

    Ok(output_file)
}
