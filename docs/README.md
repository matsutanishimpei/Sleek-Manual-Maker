# Documentation

- [State Machine](STATE_MACHINE.md): 録画、停止、キャンセル、レビューの状態遷移。
- [Features](FEATURES.md): 機能一覧。
- [JSONL Usage](JSONL_USAGE.md): セッションログ形式と利用方法。
- [Performance](PERFORMANCE.md): パフォーマンス設計メモ。
- [Release Process](RELEASE.md): GitHub Actionsによる自動リリース手順。
- [Changelog](CHANGELOG.md): 変更履歴。

## Development Checks

The CI workflow runs the following checks on Windows:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Release packages can be built with:

```powershell
.\scripts\build_release_zip.ps1
```

Debug packages can be built with:

```powershell
.\scripts\build_debug_package.ps1
```
