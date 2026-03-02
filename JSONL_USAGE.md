# 操作ログ (JSONL) の使い方

## 概要

`session_log.jsonl` は、PC操作の履歴を記録したファイルです。各行が1つのJSON オブジェクトで、操作とスクリーンショットを紐付けています。

## ファイル形式

JSONL (JSON Lines) 形式で、1行に1つのJSONオブジェクトが記録されます。

### サンプル

```jsonl
{"timestamp":"2026-03-02T14:05:30.123456+09:00","action":"MouseClick","x":1234,"y":567,"target_monitor_id":1,"image_path":"image_001.png"}
{"timestamp":"2026-03-02T14:05:32.456789+09:00","action":"KeyPress_Tab","x":null,"y":null,"target_monitor_id":1,"image_path":"image_002.png"}
{"timestamp":"2026-03-02T14:05:35.789012+09:00","action":"MouseClick","x":2048,"y":768,"target_monitor_id":2,"image_path":"image_003.png"}
```

### フィールド説明

| フィールド | 型 | 説明 | 例 |
|-----------|-----|------|-----|
| `timestamp` | String | ISO 8601形式のタイムスタンプ（マイクロ秒精度） | `"2026-01-30T14:05:30.123456+09:00"` |
| `action` | String | 操作種別 | `"MouseClick"` または `"KeyPress_Tab"` |
| `x` | Number or null | マウスのX座標（Tabキーの場合はnull） | `1234` |
| `y` | Number or null | マウスのY座標（Tabキーの場合はnull） | `567` |
| `target_monitor_id` | Number | キャプチャしたモニターのID | `1` |
| `image_path` | String | 保存された画像の相対パス（フォルダ内） | `"image_001.png"` |

## 活用例

### 1. Pythonでの読み込み

```python
import json

# JSONLファイルを読み込み
with open('session_log.jsonl', 'r', encoding='utf-8') as f:
    operations = [json.loads(line) for line in f]

# 操作履歴を表示
for op in operations:
    print(f"{op['timestamp']}: {op['action']} at ({op['x']}, {op['y']}) -> {op['image_path']}")
```

### 2. マニュアル生成

```python
import json
from datetime import datetime

def generate_manual(jsonl_path, output_md):
    with open(jsonl_path, 'r', encoding='utf-8') as f:
        operations = [json.loads(line) for line in f]
    
    with open(output_md, 'w', encoding='utf-8') as md:
        md.write("# 操作マニュアル\n\n")
        
        for i, op in enumerate(operations, 1):
            timestamp = datetime.fromisoformat(op['timestamp'])
            md.write(f"## ステップ {i}: {op['action']}\n\n")
            md.write(f"**時刻**: {timestamp.strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            
            if op['x'] is not None and op['y'] is not None:
                md.write(f"**座標**: ({op['x']}, {op['y']})\n\n")
            
            md.write(f"**モニター**: Display {op['target_monitor_id']}\n\n")
            md.write(f"![スクリーンショット]({op['image_path']})\n\n")
            md.write("---\n\n")

# 実行
generate_manual('session_log.jsonl', 'manual.md')
```

### 3. 操作分析

```python
import json
from collections import Counter
from datetime import datetime

with open('session_log.jsonl', 'r', encoding='utf-8') as f:
    operations = [json.loads(line) for line in f]

# 操作種別の集計
action_counts = Counter(op['action'] for op in operations)
print("操作種別の統計:")
for action, count in action_counts.items():
    print(f"  {action}: {count}回")

# モニター別の集計
monitor_counts = Counter(op['target_monitor_id'] for op in operations)
print("\nモニター別の統計:")
for monitor_id, count in monitor_counts.items():
    print(f"  Display {monitor_id}: {count}回")

# 操作間隔の分析
if len(operations) > 1:
    intervals = []
    for i in range(1, len(operations)):
        t1 = datetime.fromisoformat(operations[i-1]['timestamp'])
        t2 = datetime.fromisoformat(operations[i]['timestamp'])
        interval = (t2 - t1).total_seconds()
        intervals.append(interval)
    
    print(f"\n操作間隔の統計:")
    print(f"  平均: {sum(intervals)/len(intervals):.2f}秒")
    print(f"  最小: {min(intervals):.2f}秒")
    print(f"  最大: {max(intervals):.2f}秒")
```

### 4. jqコマンドでの抽出

```bash
# マウスクリックのみを抽出
cat session_log.jsonl | jq 'select(.action == "MouseClick")'

# Display 1での操作のみを抽出
cat session_log.jsonl | jq 'select(.target_monitor_id == 1)'

# 画像パスのリストを取得
cat session_log.jsonl | jq -r '.image_path'

# タイムスタンプと操作種別のみを表示
cat session_log.jsonl | jq '{timestamp, action}'
```

## 注意事項

1. **追記モード**: ファイルは追記モードで書き込まれるため、アプリを再起動しても履歴は保持されます
2. **ファイルサイズ**: 長時間使用するとファイルサイズが大きくなるため、定期的にバックアップ・削除を推奨
3. **文字エンコーディング**: UTF-8で保存されます
4. **タイムゾーン**: タイムスタンプにはタイムゾーン情報が含まれます（例: `+09:00`）

## ファイル管理

### ログのクリア

```bash
# Windowsの場合
Remove-Item session_log.jsonl

# または空ファイルで上書き
New-Item -ItemType File -Path session_log.jsonl -Force
```

### ログのバックアップ

```bash
# タイムスタンプ付きでバックアップ
Copy-Item session_log.jsonl "session_log_backup_$(Get-Date -Format 'yyyyMMdd_HHmmss').jsonl"
```

### ログのローテーション

```python
import os
import shutil
from datetime import datetime

def rotate_log(max_size_mb=10):
    log_file = 'session_log.jsonl'
    
    if not os.path.exists(log_file):
        return
    
    size_mb = os.path.getsize(log_file) / (1024 * 1024)
    
    if size_mb > max_size_mb:
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        backup_file = f'session_log_{timestamp}.jsonl'
        shutil.move(log_file, backup_file)
        print(f"ログをローテーションしました: {backup_file}")

# 実行
rotate_log()
```

## まとめ

`session_log.jsonl` を活用することで、以下が可能になります：

- ✅ 操作履歴の可視化
- ✅ 自動マニュアル生成
- ✅ ユーザー行動分析
- ✅ デバッグ・トラブルシューティング
- ✅ 操作の再現・検証
