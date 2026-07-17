# SleekManualMaker (PC Operation Logger)

## 概要

PC操作を自動的に監視し、特定のアクション（マウスクリックやキー入力）が発生した際にスクリーンショットを自動保存、そしてワンクリックで美しいHTML手順書を生成・編集できるツールです。

## 主な機能

### 1. 録画機能 (Recording)
- **自動操作記録:**
  - マウスの「左クリック」および「右クリック」を検知して自動的に記録します。
  - 操作した瞬間のスクリーンショットを撮影し、タイムスタンプとともに保存します。
- **マルチモニター完全対応:**
  - 複数のディスプレイがある場合、マウスカーソルが存在するディスプレイを自動的に判定してキャプチャします。
  - **DPIスケーリング対応:** Windowsの拡大率（125%, 150%など）設定環境下でも、物理ピクセルと論理ピクセルを正しく計算し、ズレのない画像を保存します。
- **パフォーマンス最適化:**
  - メインスレッドではキャプチャのみを行い、画像保存等の重い処理は別スレッドで非同期に行うため、ユーザー操作を阻害しません。

### 2. レビュー・保存機能 (Review & Save)
録画停止後、保存中のスクリーンショットとログをすべて待ってからサマリー確認画面に遷移します。
- **軽量サマリー確認:**
  - アプリ内では画像のデコード・描画を行わず、記録件数や合計ファイルサイズ（MB）のみを軽量に表示します。
  - ワンクリックでHTML手順書を非同期に生成し、ブラウザで自動起動します。不要な場合は、保存処理の完了を待ってからセッションデータを安全に破棄できます。

### 3. マニュアル自動生成・インタラクティブ編集 (HTML Generator & Editor)
生成されたHTMLファイル自体が、高度な手順書エディタ（編集ツール）として動作します。
- **ブラウザ上での一元編集:**
  - **画像プレビュー＆個別ステップ削除**: ブラウザ上で画像を確認しながら、不要なステップをその場で削除（自動リナンバリング）できます。
  - **非破壊マーカー切り替え**: クリック位置を示す赤色マーカーの表示／非表示を切り替えられます。
  - **説明文の入力・編集**: 各ステップの説明テキストを直接入力・変更できます。
  - **個人情報の黒塗りマスク**: ドラッグ操作で画像内の機密情報を部分的に黒塗りマスクできます。
- **完全自己完結型:**
  - すべての画像を **Base64形式** でHTML内に直接埋め込むため、HTMLファイル1つを配布・共有するだけでどこでも閲覧・再編集が可能です。

## ビルドと配布

### 配布用パッケージの作成
以下のPowerShellスクリプトを実行すると、`dist/` 配下に配布用zipとSHA-256ハッシュを生成します。

```powershell
.\scripts\build_release_zip.ps1
```

生成されるもの:
- `dist/SleekManualMaker.zip`: 配布用zip
- `dist/SleekManualMaker.zip.sha256`: SHA-256ハッシュ

### GitHub Releaseの自動更新

`v*` 形式のタグをpushすると、GitHub ActionsがWindows上で検査・ビルド・zip作成を行い、GitHub Releaseへ `SleekManualMaker.zip` とSHA-256ファイルをアップロードします。

```bash
git tag v2.0.1
git push origin v2.0.1
```

GitHub Actionsの `Release` workflowは手動実行にも対応しており、既存タグのリリース資産を再作成・更新できます。

### デバッグ用パッケージの作成
開発やトラブルシューティング時には、デバッグログを確認できる版を作成できます。

```bash
.\scripts\build_debug_package.ps1
```

## 実行方法

生成された `SleekManualMaker.zip` を展開し、中の `SleekManualMaker.exe` を実行してください。

録画データとログは、Windowsでは `%APPDATA%\SleekManualMaker\` 配下に保存されます。

## 開発時の品質チェック

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

GitHub Actionsでも同じ検査を実行します。

## 技術仕様

### 使用クレート
- **rdev**: キーボード・マウスイベントの監視
- **screenshots**: スクリーンキャプチャ機能
- **image**: 画像処理
- **egui / eframe**: 高速で軽量なGUIフレームワーク

### ファイル構成
- `%APPDATA%\SleekManualMaker\records\`: 録画セッションごとのデータ（画像・ログ）が保存されます。
- `%APPDATA%\SleekManualMaker\log\application.log`: アプリケーションの動作ログ。
- `docs/`: 設計メモ、状態遷移図、ログ仕様などの詳細ドキュメント。

## ドキュメント

詳細な設計資料は [docs/README.md](docs/README.md) を参照してください。

- [状態遷移図](docs/STATE_MACHINE.md)
- [JSONLログ仕様](docs/JSONL_USAGE.md)
- [パフォーマンス設計](docs/PERFORMANCE.md)
- [変更履歴](docs/CHANGELOG.md)

## ライセンス

このプロジェクトのライセンスについては、プロジェクトオーナーにお問い合わせください。
