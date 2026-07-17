@echo off
chcp 65001 > nul
setlocal

echo [BUILD] デバッグパッケージを作成しています...
echo.

:: cargo の正確なパスを判定
set CARGO_PATH=cargo
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set CARGO_PATH="%USERPROFILE%\.cargo\bin\cargo.exe"
)

:: Debugビルドを実行
%CARGO_PATH% build --bin SleekManualMaker
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] ビルドに失敗しました。
    pause
    exit /b 1
)


:: パッケージディレクトリの準備
if not exist "debug_package" mkdir "debug_package"
if not exist "debug_package\assets" mkdir "debug_package\assets"

:: ファイルのコピー
echo [COPY] ファイルをコピー中...
copy /Y "target\debug\SleekManualMaker.exe" "debug_package\" > nul
if exist "assets" (
    xcopy /Y /E "assets" "debug_package\assets\" > nul
)

:: 起動用バッチファイルの生成
echo @echo off > "debug_package\start_debug.bat"
echo chcp 65001 ^> nul >> "debug_package\start_debug.bat"
echo cd /d "%%~dp0" >> "debug_package\start_debug.bat"
echo echo [DEBUG MODE] アプリを起動します... >> "debug_package\start_debug.bat"
echo start "" "SleekManualMaker.exe" >> "debug_package\start_debug.bat"

echo.
echo [SUCCESS] デバッグパッケージ作成完了！
echo 以下のフォルダを確認してください:
echo   -> debug_package
echo.
pause
