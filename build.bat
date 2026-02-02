@echo off
chcp 65001 > nul
echo リリースビルドを開始します...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] ビルドに失敗しました。
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo [SUCCESS] ビルドが完了しました。
echo 実行ファイル: target\release\pc_operation_logger.exe
echo.
pause
