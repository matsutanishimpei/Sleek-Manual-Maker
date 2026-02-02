@echo off
chcp 65001 > nul
setlocal

echo [BUILD] リリースパッケージを作成しています...
echo.

:: Releaseビルドを実行
cargo build --release --bin SleekManualMaker
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] ビルドに失敗しました。
    pause
    exit /b 1
)

:: パッケージディレクトリの準備
if not exist "release_package" mkdir "release_package"
if not exist "release_package\assets" mkdir "release_package\assets"

:: ファイルのコピー
echo [COPY] ファイルをコピー中...
copy /Y "target\release\SleekManualMaker.exe" "release_package\" > nul
if exist "assets" (
    xcopy /Y /E "assets" "release_package\assets\" > nul
)



echo.
echo [SUCCESS] 作成完了！
echo 以下のフォルダを配布してください:
echo   -> release_package
echo.
pause
