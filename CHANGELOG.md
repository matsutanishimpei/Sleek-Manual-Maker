# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-03-02

### Added
- **GUIアプリ化**: `egui` と `eframe` を採用し、直感的なデスクトップアプリとして統合
  - アプリ内で「録画開始」「プレビュー・削除」「マニュアル生成」の全工程が完結
  - 削除したステップを反映した手順書をその場で生成可能に
- **キャンセル機能**: 録画中・レビュー中に「録画をキャンセル」ボタンを追加し、不要なセッションデータを即座に破棄可能に
- **エラー通知連携**: ディスクフル時などの画像保存エラーをGUIログに表示する機能を追加

### Changed
- **ディレクトリ構造の進化**: 録画ごとに `records/YYYYMMDD-HHMMSS/` フォルダを作成し、ログ（`session_log.jsonl`）と画像群（`image_001.png...`）を整理
- **メモリ管理の最適化**: 
  - レビュー画面（画像一覧）に `egui` の仮想スクロール（`show_rows`）を導入し、画像読み込み時やスクロール時のGPUメモリ使用量を大幅に節約
  - MouseMove 共有用のロックを `Mutex` から `AtomicU64` へ変更し、ホットパスの完全lock-free化を実現
  - バックグラウンド保存用チャネルを `sync_channel(10)` で制限し、異常時のメモリ溢れを防止
- **PNGエンコード速度向上**: バックグラウンド保存の圧縮レベルを `Fast` に変更しスループットを改善
- **GUIのCPU使用率低減**: 待機中やレビュー中の再描画更新頻度を200ms間隔に抑制
- **コンパイラ警告等修正**: 不必要な `Arc<Mutex>` の削除や、リリースビルド時の最適化設定を追加

## [0.2.0] - 2026-01-30

### Added
- **シングルモニター撮影**: マウス位置から該当モニターを自動判定し、そのモニターのみをキャプチャ
- **操作ログ記録**: JSONL形式で操作履歴を `session_log.jsonl` に記録
  - タイムスタンプ（マイクロ秒精度）
  - 操作種別（MouseClick / KeyPress_Tab）
  - マウス座標
  - 対象モニターID
  - 画像パス
- **HTML生成ツール**: `session_log.jsonl` から手順書 `manual.html` を自動生成
  - 独立したバイナリ (`src/bin/generate_html.rs`)
  - `cargo run --bin generate_html` で実行
  - 美しいデザインのHTML手順書を生成
  - 各ステップに画像を埋め込み
  - **画像をBase64形式で埋め込み**: どの環境でも画像が表示される
  - グラデーション背景、カード形式、ホバーエフェクトなど
- **DisplayInfo::contains_point メソッド**: マウス座標がディスプレイ内にあるか判定
- **JSONL_USAGE.md**: JSONLログの活用方法を記載したドキュメント

### Changed
- **パフォーマンス大幅改善**: マルチモニター結合を廃止し、シングルモニター撮影に変更
  - 処理するピクセル数を約50%削減（2モニター環境）
  - キャプチャ時間をさらに短縮（12-39ms → さらに高速化）
- **ファイル名形式**: `*_combined.png` → `*_displayX.png` に変更
- **CaptureMessage構造体**: `Vec<CaptureData>` → 単一の `CaptureData` に変更
- **process_and_save_captures関数**: `process_and_save_capture` に名称変更し、シングルキャプチャ対応

### Removed
- マルチモニター画像結合機能（パフォーマンス優先のため）
- 画像結合に関連する処理（バウンディングボックス計算、copy_from等）

### Dependencies
- Added: `serde = "1.0"` (derive feature)
- Added: `serde_json = "1.0"`

## [0.1.0] - 2026-01-30

### Added
- **初回リリース**: PC操作ログツールの基本機能
- **自動スクリーンショット取得**: マウス左クリック、Tabキー押下で自動撮影
- **マルチモニター対応**: 全モニターを結合して1枚の画像に保存
- **マウスカーソル表示**: 赤い円でカーソル位置を表示
- **DPIスケーリング対応**: 高DPI環境でも正確なサイズで保存
- **パフォーマンス最適化**:
  - Screen情報のキャッシュ（起動時に1回のみ取得）
  - 完全オンメモリ処理（ディスクI/O排除）
  - 非同期処理アーキテクチャ（チャネルベース）
- **クールダウン機能**: 500ms間隔で連続撮影を制限
- **README.md**: プロジェクトの説明とドキュメント
- **PERFORMANCE.md**: パフォーマンス最適化の詳細レポート

### Dependencies
- `rdev = "0.5.3"`: キーボード・マウスイベント監視
- `screenshots = "0.8.10"`: スクリーンキャプチャ
- `image = "0.25.9"`: 画像処理
- `chrono = "0.4"`: タイムスタンプ生成
- `anyhow = "1.0"`: エラーハンドリング

---

## Version History Summary

| Version | Date | Key Features |
|---------|------|--------------|
| 2.0.0 | 2026-03-02 | 完全GUIアプリ化、レビュー＆キャンセル機能追加、メモリ大幅最適化 |
| 0.2.0 | 2026-01-30 | シングルモニター撮影、JSONLログ記録 |
| 0.1.0 | 2026-01-30 | 初回リリース、オンメモリ処理最適化 |

---

## Migration Guide

### 0.1.0 → 0.2.0

#### ファイル名の変更
- **Before**: `screenshots/20260130-140530-123_combined.png`
- **After**: `screenshots/20260130-140530-123_display1.png`

#### 動作の変更
- **Before**: 全モニターを結合した1枚の画像
- **After**: マウス位置のモニター1枚のみ

#### 新機能の利用
- `session_log.jsonl` が自動生成されます
- 詳細は `JSONL_USAGE.md` を参照してください

#### 互換性
- 既存のスクリーンショットには影響ありません
- 新しいスクリーンショットは新形式で保存されます
