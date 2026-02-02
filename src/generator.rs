use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use crate::types::OperationLog;

/// 画像ファイルをBase64エンコードしてdata URIを生成
fn image_to_base64_data_uri(image_path: &PathBuf) -> String {
    match std::fs::read(image_path) {
        Ok(image_data) => {
            let base64_string = general_purpose::STANDARD.encode(&image_data);
            format!("data:image/png;base64,{}", base64_string)
        }
        Err(e) => {
            eprintln!("警告: 画像ファイルの読み込みに失敗しました: {:?}", image_path);
            format!("[Image error: {}]", e)
        }
    }
}

pub fn generate_html(session_folder: &PathBuf, logs: &[OperationLog]) -> Result<PathBuf> {
    let output_file = session_folder.join("manual.html");
    let mut html_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_file)?;

    let now = Local::now();
    writeln!(html_file, "<!DOCTYPE html>")?;
    writeln!(html_file, "<html lang=\"ja\">")?;
    writeln!(html_file, "<head>")?;
    writeln!(html_file, "    <meta charset=\"UTF-8\">")?;
    writeln!(html_file, "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(html_file, "    <title>操作手順書</title>")?;
    writeln!(html_file, "    <style>")?;
    writeln!(html_file, "        :root {{ --primary: #667eea; --danger: #e53e3e; --text: #2d3748; --bg: #f7fafc; }}")?;
    writeln!(html_file, "        * {{ margin: 0; padding: 0; box-sizing: border-box; }}")?;
    writeln!(html_file, "        body {{ font-family: 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); padding: 40px 20px; line-height: 1.6; }}")?;
    
    writeln!(html_file, "        .container {{ max-width: 1000px; margin: 0 auto; }}")?;
    writeln!(html_file, "        .header {{ text-align: center; margin-bottom: 40px; }}")?;
    writeln!(html_file, "        .header h1 {{ font-size: 2.5rem; color: #4a5568; margin-bottom: 10px; }}")?;
    writeln!(html_file, "        .meta-info {{ color: #718096; font-size: 0.9rem; }}")?;

    writeln!(html_file, "        .step-card {{ background: white; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.05); margin-bottom: 30px; overflow: hidden; transition: transform 0.2s; border: 1px solid #edf2f7; }}")?;
    writeln!(html_file, "        .step-card:hover {{ box-shadow: 0 10px 15px rgba(0,0,0,0.1); }}")?;

    // Header Bar
    writeln!(html_file, "        .card-header {{ background: #f8fafc; padding: 15px 25px; border-bottom: 1px solid #edf2f7; display: flex; justify-content: space-between; align-items: center; }}")?;
    writeln!(html_file, "        .step-badge {{ background: var(--primary); color: white; padding: 4px 12px; border-radius: 20px; font-weight: bold; font-size: 0.9rem; }}")?;
    writeln!(html_file, "        .controls {{ display: flex; gap: 10px; }}")?;
    writeln!(html_file, "        .btn {{ padding: 6px 12px; border-radius: 6px; border: 1px solid #cbd5e0; background: white; cursor: pointer; font-size: 0.85rem; transition: all 0.2s; }}")?;
    writeln!(html_file, "        .btn:hover {{ background: #edf2f7; }}")?;
    writeln!(html_file, "        .btn-toggle.active {{ background: #ebf8ff; border-color: #4299e1; color: #2b6cb0; }}")?;
    writeln!(html_file, "        .btn-delete {{ color: var(--danger); border-color: #feb2b2; }}")?;
    writeln!(html_file, "        .btn-delete:hover {{ background: #fff5f5; }}")?;

    // Card Body
    writeln!(html_file, "        .card-body {{ display: flex; flex-wrap: wrap; }}")?;
    
    // Left: Image
    writeln!(html_file, "        .image-area {{ flex: 2; min-width: 300px; padding: 20px; border-right: 1px solid #edf2f7; position: relative; }}")?;
    writeln!(html_file, "        .image-container {{ position: relative; width: 100%; border-radius: 6px; overflow: hidden; border: 1px solid #e2e8f0; line-height: 0; }}")?;
    writeln!(html_file, "        .screenshot {{ width: 100%; height: auto; display: block; }}")?;
    
    // Marker
    writeln!(html_file, "        @keyframes ripple {{ 0% {{ transform: translate(-50%, -50%) scale(1); opacity: 1; }} 100% {{ transform: translate(-50%, -50%) scale(3); opacity: 0; }} }}")?;
    writeln!(html_file, "        .marker {{ position: absolute; width: 30px; height: 30px; border: 3px solid rgba(255, 0, 0, 0.8); background-color: rgba(255, 0, 0, 0.2); border-radius: 50%; transform: translate(-50%, -50%); pointer-events: none; z-index: 10; box-shadow: 0 0 10px rgba(255,0,0,0.5); }}")?;
    writeln!(html_file, "        .marker::before {{ content: ''; position: absolute; top: 50%; left: 50%; width: 100%; height: 100%; border: 2px solid rgba(255, 0, 0, 0.8); border-radius: 50%; animation: ripple 1.5s infinite ease-out; box-sizing: border-box; }}")?;
    writeln!(html_file, "        .marker::after {{ content: ''; position: absolute; top: 50%; left: 50%; width: 6px; height: 6px; background-color: red; border-radius: 50%; transform: translate(-50%, -50%); }}")?;

    // Right: Description
    writeln!(html_file, "        .desc-area {{ flex: 1; min-width: 250px; padding: 25px; display: flex; flex-direction: column; }}")?;
    writeln!(html_file, "        .action-title {{ font-size: 1.1rem; font-weight: 600; color: #2d3748; margin-bottom: 15px; border-bottom: 2px solid #edf2f7; padding-bottom: 10px; }}")?;
    writeln!(html_file, "        .description-box {{ flex-grow: 1; padding: 15px; border: 1px solid #e2e8f0; border-radius: 6px; background: #ffffb020; min-height: 100px; outline: none; transition: border-color 0.2s; color: #4a5568; white-space: pre-wrap; }}")?;
    writeln!(html_file, "        .description-box:focus {{ border-color: var(--primary); background: white; box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1); }}")?;
    writeln!(html_file, "        .description-box:empty:before {{ content: 'ここに手順の説明を入力...'; color: #a0aec0; }}")?;

    // Footer
    writeln!(html_file, "        .save-bar {{ position: fixed; bottom: 20px; right: 20px; z-index: 100; }}")?;
    writeln!(html_file, "        .btn-save {{ background: var(--primary); color: white; padding: 15px 30px; border-radius: 50px; border: none; font-weight: bold; font-size: 1.1rem; box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4); cursor: pointer; transition: transform 0.2s; }}")?;
    writeln!(html_file, "        .btn-save:hover {{ transform: translateY(-2px); box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5); }}")?;
    writeln!(html_file, "        .btn-save:active {{ transform: translateY(0); }}")?;

    writeln!(html_file, "        @media (max-width: 768px) {{ .card-body {{ flex-direction: column; }} .image-area {{ border-right: none; border-bottom: 1px solid #edf2f7; }} }}")?;
    writeln!(html_file, "    </style>")?;
    writeln!(html_file, "</head>")?;
    writeln!(html_file, "<body>")?;
    
    writeln!(html_file, "    <div class=\"container\">")?;
    writeln!(html_file, "        <div class=\"header\">")?;
    writeln!(html_file, "            <h1>操作手順書</h1>")?;
    writeln!(html_file, "            <p class=\"meta-info\">生成日時: {} | ステップ数: {}</p>", now.format("%Y/%m/%d %H:%M"), logs.len())?;
    writeln!(html_file, "        </div>")?;

    for (i, log) in logs.iter().enumerate() {
        let step_num = i + 1;
        let step_id = format!("step-{}", step_num);
        
        writeln!(html_file, "        <div class=\"step-card\" id=\"{}\">", step_id)?;
        
        // Header
        writeln!(html_file, "            <div class=\"card-header\">")?;
        writeln!(html_file, "                <div class=\"step-badge\">STEP {}</div>", step_num)?;
        writeln!(html_file, "                <div class=\"controls\">")?;
        writeln!(html_file, "                    <button class=\"btn btn-toggle active\" onclick=\"toggleDot('{}', this)\">🔴 赤点</button>", step_id)?;
        writeln!(html_file, "                    <button class=\"btn btn-delete\" onclick=\"deleteStep('{}')\">🗑️ 削除</button>", step_id)?;
        writeln!(html_file, "                </div>")?;
        writeln!(html_file, "            </div>")?;

        // Body
        writeln!(html_file, "            <div class=\"card-body\">")?;
        
        // Image Area
        writeln!(html_file, "                <div class=\"image-area\">")?;
        
        let image_path = session_folder.join(&log.image_path);
        if image_path.exists() {
                let (real_w, real_h) = image::image_dimensions(&image_path).unwrap_or((0, 0));
                let data_uri = image_to_base64_data_uri(&image_path);
                
                writeln!(html_file, "                    <div class=\"image-container\">")?;
                writeln!(html_file, "                        <img src=\"{}\" class=\"screenshot\">", data_uri)?;
                
                // Marker
                if let (Some(x), Some(y)) = (log.x, log.y) {
                    if real_w > 0 && real_h > 0 {
                        let left_percent = ((x as f64 / real_w as f64) * 100.0).clamp(0.0, 100.0);
                        let top_percent = ((y as f64 / real_h as f64) * 100.0).clamp(0.0, 100.0);
                        writeln!(html_file, "                        <div class=\"marker\" style=\"left: {:.2}%; top: {:.2}%;\"></div>", left_percent, top_percent)?;
                    } else if let (Some(w), Some(h)) = (log.width, log.height) {
                        if w > 0 && h > 0 {
                            let left_percent = ((x as f64 / w as f64) * 100.0).clamp(0.0, 100.0);
                            let top_percent = ((y as f64 / h as f64) * 100.0).clamp(0.0, 100.0);
                            writeln!(html_file, "                        <div class=\"marker\" style=\"left: {:.2}%; top: {:.2}%;\"></div>", left_percent, top_percent)?;
                        }
                    }
                }
                writeln!(html_file, "                    </div>")?;
        } else {
            writeln!(html_file, "                    <div style=\"color:red; padding: 20px;\">画像が見つかりません</div>")?;
        }
        writeln!(html_file, "                </div>")?;

        // Description Area
        writeln!(html_file, "                <div class=\"desc-area\">")?;
        writeln!(html_file, "                    <div class=\"action-title\">{}</div>", log.action)?;
        writeln!(html_file, "                    <div class=\"description-box\" contenteditable=\"true\"></div>")?;
        writeln!(html_file, "                </div>")?;
        
        writeln!(html_file, "            </div>")?; // End card-body
        writeln!(html_file, "        </div>")?; // End step-card
    }

    writeln!(html_file, "    </div>")?;
    
    // Save Button
    writeln!(html_file, "    <div class=\"save-bar\">")?;
    writeln!(html_file, "        <button class=\"btn-save\" onclick=\"saveHtml()\">💾 手順書を保存</button>")?;
    writeln!(html_file, "    </div>")?;

    // Scripts
    writeln!(html_file, "    <script>")?;
    writeln!(html_file, "        function toggleDot(stepId, btn) {{")?;
    writeln!(html_file, "            const step = document.getElementById(stepId);")?;
    writeln!(html_file, "            const marker = step.querySelector('.marker');")?;
    writeln!(html_file, "            if (marker) {{")?;
    writeln!(html_file, "                const isHidden = marker.style.display === 'none';")?;
    writeln!(html_file, "                marker.style.display = isHidden ? 'block' : 'none';")?;
    writeln!(html_file, "                btn.classList.toggle('active', isHidden);")?;
    writeln!(html_file, "                btn.innerText = isHidden ? '🔴 赤点' : '⚪ 赤点なし';")?;
    writeln!(html_file, "            }}")?;
    writeln!(html_file, "        }}")?;
    
    writeln!(html_file, "        function deleteStep(stepId) {{")?;
    writeln!(html_file, "            if(confirm('このステップを削除してもよろしいですか？')) {{")?;
    writeln!(html_file, "                const el = document.getElementById(stepId);")?;
    writeln!(html_file, "                if(el) el.remove();")?;
    writeln!(html_file, "            }}")?;
    writeln!(html_file, "        }}")?;

    writeln!(html_file, "        function saveHtml() {{")?;
    writeln!(html_file, "            const htmlContent = '<!DOCTYPE html>\\n' + document.documentElement.outerHTML;")?;
    writeln!(html_file, "            const blob = new Blob([htmlContent], {{ type: 'text/html' }});")?;
    writeln!(html_file, "            const a = document.createElement('a');")?;
    writeln!(html_file, "            a.href = URL.createObjectURL(blob);")?;
    writeln!(html_file, "            a.download = 'manual_edited.html';")?;
    writeln!(html_file, "            a.click();")?;
    writeln!(html_file, "        }}")?;
    writeln!(html_file, "    </script>")?;

    writeln!(html_file, "</body>")?;
    writeln!(html_file, "</html>")?;

    Ok(output_file)
}
