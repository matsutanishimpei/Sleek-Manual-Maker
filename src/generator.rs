use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::types::OperationLog;

// ─────────────────────────────────────────────
// CSS / JS 定数
// ─────────────────────────────────────────────

const HTML_CSS: &str = r##"
        :root { --primary: #667eea; --danger: #e53e3e; --text: #2d3748; --bg: #f7fafc; }
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); padding: 40px 20px; line-height: 1.6; }
        .container { max-width: 1000px; margin: 0 auto; }
        .header { text-align: center; margin-bottom: 40px; }
        .header h1 { font-size: 2.5rem; color: #4a5568; margin-bottom: 10px; }
        .meta-info { color: #718096; font-size: 0.9rem; }
        
        /* ドラッグ並び替えのスタイル */
        .step-card { background: white; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.05); margin-bottom: 30px; overflow: hidden; transition: transform 0.2s; border: 1px solid #edf2f7; cursor: grab; }
        .step-card:active { cursor: grabbing; }
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
        
        /* アクティブウィンドウバッジ */
        .window-badge {
            background: #edf2f7;
            color: #4a5568;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 0.85rem;
            margin-right: 8px;
            border: 1px solid #cbd5e0;
            font-weight: normal;
        }

        .action-title { font-size: 1.1rem; font-weight: 600; color: #2d3748; margin-bottom: 15px; border-bottom: 2px solid #edf2f7; padding-bottom: 10px; display: flex; align-items: center; flex-wrap: wrap; gap: 4px; }
        .description-box { flex-grow: 1; padding: 15px; border: 1px solid #e2e8f0; border-radius: 6px; background: #ffffb020; min-height: 100px; outline: none; transition: border-color 0.2s; color: #4a5568; white-space: pre-wrap; }
        .description-box:focus { border-color: var(--primary); background: white; box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1); }
        .description-box:empty:before { content: 'ここに手順の説明を入力...'; color: #a0aec0; }
        
        .save-bar { position: fixed; bottom: 20px; right: 20px; z-index: 1000; }
        .btn-save { background: var(--primary); color: white; padding: 15px 25px; border-radius: 50px; border: none; font-weight: bold; font-size: 1.0rem; box-shadow: 0 4px 15px rgba(102, 126, 234, 0.4); cursor: pointer; transition: transform 0.2s; }
        .btn-save:hover { transform: translateY(-2px); box-shadow: 0 6px 20px rgba(102, 126, 234, 0.5); }
        .btn-save:active { transform: translateY(0); }
        
        /* 個人情報マスク用のスタイル */
        .mask-box {
            position: absolute;
            background: black;
            border: 1px dashed white;
            cursor: move;
            z-index: 100;
        }
        .mask-delete-btn {
            position: absolute;
            top: -10px;
            right: -10px;
            background: red;
            color: white;
            border: none;
            border-radius: 50%;
            width: 20px;
            height: 20px;
            font-size: 12px;
            cursor: pointer;
            line-height: 20px;
            text-align: center;
            font-weight: bold;
        }

        @media (max-width: 768px) { .card-body { flex-direction: column; } .image-area { border-right: none; border-bottom: 1px solid #edf2f7; } }

        /* 印刷・PDF出力用メディアクエリ */
        @media print {
            body { padding: 0; background: white; color: black; }
            .container { max-width: 100%; }
            .step-card { page-break-inside: avoid; box-shadow: none; border: 1px solid #ccc; margin-bottom: 20px; cursor: default; }
            .save-bar, .controls, .mask-delete-btn { display: none !important; }
            .description-box { border: none !important; background: transparent !important; padding: 0 !important; }
            .zoom-circle { border-color: black !important; }
        }
"##;

const HTML_JS: &str = r##"
        let maskMode = false;

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
                const cards = document.querySelectorAll('.step-card');
                cards.forEach(function(card, index) {
                    const badge = card.querySelector('.step-badge');
                    if(badge) badge.textContent = 'STEP ' + (index + 1);
                });
            }
        }

        function toggleMaskMode(btn) {
            maskMode = !maskMode;
            btn.innerText = maskMode ? "🔓 マスクモード: ON" : "🔒 マスクモード: OFF";
            btn.style.background = maskMode ? "#e53e3e" : "#e2e8f0";
            btn.style.color = maskMode ? "white" : "#4a5568";
            
            document.querySelectorAll('.image-container').forEach(container => {
                container.style.cursor = maskMode ? "crosshair" : "default";
            });
        }

        function setupMaskDragging(mask, delBtn) {
            let isDragging = false;
            let startX, startY;
            let startLeft, startTop;

            mask.addEventListener('mousedown', (e) => {
                if (delBtn && e.target === delBtn) return;
                e.stopPropagation();
                isDragging = true;
                startX = e.clientX;
                startY = e.clientY;
                startLeft = parseInt(mask.style.left) || 0;
                startTop = parseInt(mask.style.top) || 0;
            });

            document.addEventListener('mousemove', (e) => {
                if (!isDragging) return;
                const dx = e.clientX - startX;
                const dy = e.clientY - startY;
                mask.style.left = (startLeft + dx) + 'px';
                mask.style.top = (startTop + dy) + 'px';
            });

            document.addEventListener('mouseup', () => {
                isDragging = false;
            });
        }

        function createMaskBox(container, x, y, width, height) {
            const mask = document.createElement('div');
            mask.className = 'mask-box';
            mask.style.left = x + 'px';
            mask.style.top = y + 'px';
            mask.style.width = width + 'px';
            mask.style.height = height + 'px';

            const delBtn = document.createElement('button');
            delBtn.className = 'mask-delete-btn';
            delBtn.innerHTML = '×';
            delBtn.onclick = (e) => {
                e.stopPropagation();
                mask.remove();
            };
            mask.appendChild(delBtn);

            setupMaskDragging(mask, delBtn);
            container.appendChild(mask);
        }

        function saveHtml() {
            const htmlContent = '<!DOCTYPE html>\n' + document.documentElement.outerHTML;
            const blob = new Blob([htmlContent], { type: 'text/html' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'manual_edited.html';
            a.click();
        }

        function exportCleanHtml() {
            const docClone = document.documentElement.cloneNode(true);
            
            docClone.querySelectorAll('.controls').forEach(el => el.remove());
            const saveBar = docClone.querySelector('.save-bar');
            if (saveBar) saveBar.remove();
            
            docClone.querySelectorAll('.mask-delete-btn').forEach(el => el.remove());
            docClone.querySelectorAll('.mask-box').forEach(el => {
                el.style.border = 'none';
                el.style.cursor = 'default';
            });
            
            docClone.querySelectorAll('.description-box').forEach(el => {
                el.removeAttribute('contenteditable');
                if (el.textContent.trim() === '') {
                    el.style.display = 'none';
                } else {
                    el.style.border = 'none';
                    el.style.background = 'transparent';
                    el.style.padding = '0';
                    el.style.boxShadow = 'none';
                }
            });

            const cssStyle = docClone.querySelector('style');
            if (cssStyle) {
                cssStyle.innerHTML += '\n.description-box { color: #2d3748; font-size: 1.05rem; }';
            }
            
            const htmlContent = '<!DOCTYPE html>\n' + docClone.outerHTML;
            const blob = new Blob([htmlContent], { type: 'text/html' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'manual_published.html';
            a.click();
        }

        function exportMarkdown() {
            let md = '# 操作手順書\n\n';
            md += `生成日時: ${new Date().toLocaleString()}\n\n`;
            
            document.querySelectorAll('.step-card').forEach((card, index) => {
                const stepNum = index + 1;
                
                const titleNode = card.querySelector('.action-title');
                let title = "";
                if (titleNode) {
                    title = Array.from(titleNode.childNodes)
                        .filter(node => node.nodeType === Node.TEXT_NODE)
                        .map(node => node.textContent.trim())
                        .join("");
                    
                    const badge = titleNode.querySelector('.window-badge');
                    if (badge) {
                        title = badge.textContent.trim() + " " + title;
                    }
                }
                
                const desc = card.querySelector('.description-box').textContent.trim();
                const imgPath = card.getAttribute('data-image-path');
                
                md += `## STEP ${stepNum}: ${title}\n\n`;
                if (desc) {
                    md += `${desc}\n\n`;
                }
                if (imgPath) {
                    md += `![STEP ${stepNum}](${imgPath})\n\n`;
                }
                md += '---\n\n';
            });
            
            const blob = new Blob([md], { type: 'text/markdown;charset=utf-8' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'manual.md';
            a.click();
        }

        let draggedCard = null;
        function initDragAndDrop() {
            const cards = document.querySelectorAll('.step-card');
            cards.forEach(card => {
                card.setAttribute('draggable', 'true');
                card.addEventListener('dragstart', (e) => {
                    if (maskMode) {
                        e.preventDefault();
                        return;
                    }
                    draggedCard = card;
                    card.style.opacity = '0.5';
                });
                card.addEventListener('dragend', (e) => {
                    card.style.opacity = '1';
                });
                card.addEventListener('dragover', (e) => {
                    e.preventDefault();
                });
                card.addEventListener('drop', (e) => {
                    e.preventDefault();
                    if (draggedCard && draggedCard !== card) {
                        const parent = card.parentNode;
                        const rect = card.getBoundingClientRect();
                        const next = (e.clientY - rect.top) / (rect.bottom - rect.top) > 0.5;
                        parent.insertBefore(draggedCard, next ? card.nextSibling : card);
                        
                        document.querySelectorAll('.step-card').forEach((c, idx) => {
                            c.querySelector('.step-badge').textContent = 'STEP ' + (idx + 1);
                        });
                    }
                });
            });
        }

        document.addEventListener('DOMContentLoaded', () => {
            document.querySelectorAll('.mask-box').forEach(mask => {
                const delBtn = mask.querySelector('.mask-delete-btn');
                setupMaskDragging(mask, delBtn);
            });

            document.querySelectorAll('.image-container').forEach(container => {
                container.addEventListener('click', (e) => {
                    if (!maskMode) return;
                    if (e.target.classList.contains('mask-delete-btn')) return;

                    const rect = container.getBoundingClientRect();
                    const x = e.clientX - rect.left;
                    const y = e.clientY - rect.top;

                    createMaskBox(container, x - 50, y - 25, 100, 50);
                });
            });

            initDragAndDrop();
        });
"##;

// ─────────────────────────────────────────────
// ヘルパー
// ─────────────────────────────────────────────

fn image_to_base64_data_uri(image_path: &PathBuf) -> String {
    match std::fs::read(image_path) {
        Ok(image_data) => {
            format!(
                "data:image/jpeg;base64,{}",
                general_purpose::STANDARD.encode(&image_data)
            )
        }
        Err(e) => {
            eprintln!(
                "警告: 画像ファイルの読み込みに失敗しました: {:?}",
                image_path
            );
            format!("[Image error: {}]", e)
        }
    }
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn write_step_card(
    f: &mut impl Write,
    session_folder: &Path,
    log: &OperationLog,
    step_num: usize,
) -> Result<()> {
    let step_id = format!("step-{}", step_num);
    let escaped_image_path = escape_html(&log.image_path);

    writeln!(
        f,
        "        <div class=\"step-card\" id=\"{}\" data-image-path=\"{}\">",
        step_id, escaped_image_path
    )?;
    writeln!(f, "            <div class=\"card-header\">")?;
    writeln!(
        f,
        "                <div class=\"step-badge\">STEP {}</div>",
        step_num
    )?;
    writeln!(f, "                <div class=\"controls\">")?;
    writeln!(f, "                    <button class=\"btn btn-toggle active\" onclick=\"toggleDot('{}', this)\">🔴 赤点</button>", step_id)?;
    writeln!(f, "                    <button class=\"btn btn-delete\" onclick=\"deleteStep('{}')\">🗑️ 削除</button>", step_id)?;
    writeln!(f, "                </div>")?;
    writeln!(f, "            </div>")?;

    writeln!(f, "            <div class=\"card-body\">")?;

    writeln!(f, "                <div class=\"image-area\">")?;
    let image_path = session_folder.join(&log.image_path);
    let data_uri = if image_path.exists() {
        Some(image_to_base64_data_uri(&image_path))
    } else {
        None
    };

    if let Some(ref uri) = data_uri {
        writeln!(f, "                    <div class=\"image-container\">")?;
        writeln!(
            f,
            "                        <img src=\"{}\" class=\"screenshot\">",
            uri
        )?;

        if let (Some(x), Some(y)) = (log.x, log.y) {
            let (dim_w, dim_h) = match (log.width, log.height) {
                (Some(w), Some(h)) if w > 0 && h > 0 => (w as f64, h as f64),
                _ => {
                    let (w, h) = image::image_dimensions(&image_path).unwrap_or((0, 0));
                    (w as f64, h as f64)
                }
            };
            if dim_w > 0.0 {
                let left = ((x as f64 / dim_w) * 100.0).clamp(0.0, 100.0);
                let top = ((y as f64 / dim_h) * 100.0).clamp(0.0, 100.0);
                writeln!(f, "                        <div class=\"marker\" style=\"left: {:.2}%; top: {:.2}%;\"></div>", left, top)?;
            }
        }
        writeln!(f, "                    </div>")?;
    } else {
        writeln!(f, "                    <div style=\"color:red; padding: 20px;\">画像が見つかりません</div>")?;
    }
    writeln!(f, "                </div>")?;

    writeln!(f, "                <div class=\"desc-area\">")?;
    let friendly_action = if log.action.starts_with("MouseClick") {
        if log.action.contains("入力: ") {
            let typed = log.action.replace("MouseClick ", "");
            format!("左クリック {}", typed)
        } else {
            "左クリック".to_string()
        }
    } else if log.action.starts_with("RightClick") {
        if log.action.contains("入力: ") {
            let typed = log.action.replace("RightClick ", "");
            format!("右クリック {}", typed)
        } else {
            "右クリック".to_string()
        }
    } else if log.action.starts_with("KeyPress_Enter") {
        if log.action.contains("入力: ") {
            let typed = log.action.replace("KeyPress_Enter ", "");
            format!("Enter入力 {}", typed)
        } else {
            "Enter入力".to_string()
        }
    } else if log.action.starts_with("KeyPress_Tab") {
        if log.action.contains("入力: ") {
            let typed = log.action.replace("KeyPress_Tab ", "");
            format!("Tabキー入力 {}", typed)
        } else {
            "Tabキー入力".to_string()
        }
    } else {
        log.action.clone()
    };

    let title_html = if let Some(ref w_title) = log.window_title {
        let truncated_title = if w_title.chars().count() > 24 {
            let s: String = w_title.chars().take(22).collect();
            format!("{}...", s)
        } else {
            w_title.clone()
        };
        format!(
            "<span class=\"window-badge\">💻 {}</span>{}",
            escape_html(&truncated_title),
            escape_html(&friendly_action)
        )
    } else {
        escape_html(&friendly_action)
    };

    writeln!(
        f,
        "                    <div class=\"action-title\">{}</div>",
        title_html
    )?;

    // クリック位置のズーム表示（画像が存在し、座標情報がある場合のみ）
    if let (Some(ref uri), Some(x), Some(y)) = (&data_uri, log.x, log.y) {
        let dim_w = match log.width {
            Some(w) if w > 0 => w as f64,
            _ => {
                let (w, _) = image::image_dimensions(&image_path).unwrap_or((0, 0));
                w as f64
            }
        };
        if dim_w > 0.0 {
            let scale = 4.0; // 4倍ズーム
            let zoom_w = 120.0 * scale;
            let ratio = zoom_w / dim_w;
            let x_scaled = x as f64 * ratio;
            let y_scaled = y as f64 * ratio;
            let left_offset = 60.0 - x_scaled;
            let top_offset = 60.0 - y_scaled;

            writeln!(f, "                    <div style=\"margin-bottom: 15px; display: flex; align-items: center; gap: 15px;\">")?;
            writeln!(f, "                        <div class=\"zoom-circle\" style=\"width: 120px; height: 120px; border-radius: 50%; border: 3px solid var(--primary); overflow: hidden; position: relative; box-shadow: 0 4px 10px rgba(0,0,0,0.15); background: #eee; flex-shrink: 0;\">")?;
            writeln!(f, "                            <img src=\"{}\" style=\"position: absolute; max-width: none; width: {:.1}px; height: auto; left: {:.1}px; top: {:.1}px; pointer-events: none;\">", uri, zoom_w, left_offset, top_offset)?;
            writeln!(f, "                            <div style=\"position: absolute; left: 57px; top: 57px; width: 6px; height: 6px; background: red; border-radius: 50%; box-shadow: 0 0 4px rgba(255,0,0,0.8);\"></div>")?;
            writeln!(f, "                        </div>")?;
            writeln!(
                f,
                "                        <div style=\"font-size: 0.85rem; color: #718096;\">"
            )?;
            writeln!(
                f,
                "                            <strong>クリック箇所の拡大</strong><br>"
            )?;
            writeln!(f, "                            座標: ({}, {})", x, y)?;
            writeln!(f, "                        </div>")?;
            writeln!(f, "                    </div>")?;
        }
    }

    writeln!(
        f,
        "                    <div class=\"description-box\" contenteditable=\"true\"></div>"
    )?;
    writeln!(f, "                </div>")?;

    writeln!(f, "            </div>")?;
    writeln!(f, "        </div>")?;
    Ok(())
}

pub fn generate_html(session_folder: &Path, logs: &[OperationLog]) -> Result<PathBuf> {
    let output_file = session_folder.join("manual.html");
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_file)?;

    let now = Local::now();

    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html lang=\"ja\">")?;
    writeln!(f, "<head>")?;
    writeln!(f, "    <meta charset=\"UTF-8\">")?;
    writeln!(
        f,
        "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
    )?;
    writeln!(f, "    <title>操作手順書</title>")?;
    writeln!(f, "    <style>{}</style>", HTML_CSS)?;
    writeln!(f, "</head>")?;
    writeln!(f, "<body>")?;

    writeln!(f, "    <div class=\"container\">")?;
    writeln!(f, "        <div class=\"header\">")?;
    writeln!(f, "            <h1>操作手順書</h1>")?;
    writeln!(
        f,
        "            <p class=\"meta-info\">生成日時: {} | ステップ数: {}</p>",
        now.format("%Y/%m/%d %H:%M"),
        logs.len()
    )?;
    writeln!(f, "        </div>")?;

    for (i, log) in logs.iter().enumerate() {
        write_step_card(&mut f, session_folder, log, i + 1)?;
    }

    writeln!(f, "    </div>")?;

    writeln!(
        f,
        "    <div class=\"save-bar\" style=\"display: flex; gap: 10px;\">"
    )?;
    writeln!(f, "        <button class=\"btn-save\" style=\"background: #e2e8f0; color: #4a5568; box-shadow: none;\" onclick=\"toggleMaskMode(this)\">🔒 マスクモード: OFF</button>")?;
    writeln!(f, "        <button class=\"btn-save\" style=\"background: #718096; box-shadow: none;\" onclick=\"saveHtml()\">💾 編集用HTMLを保存 (一時保存)</button>")?;
    writeln!(f, "        <button class=\"btn-save\" style=\"background: #319795; box-shadow: none;\" onclick=\"exportMarkdown()\">📤 Markdown出力</button>")?;
    writeln!(f, "        <button class=\"btn-save\" onclick=\"exportCleanHtml()\">📤 配布用HTMLを出力 (クリーン版)</button>")?;
    writeln!(f, "    </div>")?;

    writeln!(f, "    <script>{}</script>", HTML_JS)?;

    writeln!(f, "</body>")?;
    writeln!(f, "</html>")?;

    Ok(output_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_escapes_markup_and_quotes() {
        assert_eq!(
            escape_html("<script>alert('x') & \"y\"</script>"),
            "&lt;script&gt;alert(&#39;x&#39;) &amp; &quot;y&quot;&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_html_leaves_normal_japanese_text_intact() {
        assert_eq!(
            escape_html("左クリック 入力テキスト"),
            "左クリック 入力テキスト"
        );
    }
}
