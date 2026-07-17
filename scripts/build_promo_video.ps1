param(
    [string]$OutputPath = "docs/site/assets/promo-video.mp4"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$assetDir = Join-Path $repoRoot "docs/site/assets"
$subtitlePath = Join-Path $repoRoot "docs/video/promo-subtitles.ass"
$narrationPath = Join-Path $repoRoot "docs/video/promo-narration.txt"
$workDir = Join-Path $repoRoot "target/promo-video"
$output = Join-Path $repoRoot $OutputPath

$requiredFiles = @(
    "promo-presenter.png",
    "app-idle.png",
    "app-recording.png",
    "app-summary.png",
    "manual-output.png"
)

foreach ($file in $requiredFiles) {
    $path = Join-Path $assetDir $file
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required asset not found: $path"
    }
}

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    throw "ffmpeg is required to build the promo video."
}

New-Item -ItemType Directory -Force -Path $workDir | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null

$voicePath = Join-Path $workDir "narration.wav"
$musicPath = Join-Path $workDir "music.wav"
$silentVideoPath = Join-Path $workDir "silent-video.mp4"

Add-Type -AssemblyName System.Speech
$voice = New-Object System.Speech.Synthesis.SpeechSynthesizer
try {
    $voice.SelectVoice("Microsoft Haruka Desktop")
    $voice.Rate = 0
    $voice.Volume = 100
    $voice.SetOutputToWaveFile($voicePath)
    $narration = [System.IO.File]::ReadAllText($narrationPath, [System.Text.Encoding]::UTF8)
    $voice.Speak($narration)
}
finally {
    $voice.Dispose()
}

$presenter = Join-Path $assetDir "promo-presenter.png"
$idle = Join-Path $assetDir "app-idle.png"
$recording = Join-Path $assetDir "app-recording.png"
$summary = Join-Path $assetDir "app-summary.png"
$manual = Join-Path $assetDir "manual-output.png"

$filter = @"
[0:v]scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,zoompan=z='min(zoom+0.00018,1.022)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=120:s=1920x1080:fps=30,setsar=1[v0];
[1:v]scale=1720:968:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0xF4F7F8,zoompan=z='min(zoom+0.00012,1.018)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=150:s=1920x1080:fps=30,setsar=1[v1];
[2:v]scale=1720:968:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0xF4F7F8,zoompan=z='min(zoom+0.00012,1.018)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=150:s=1920x1080:fps=30,setsar=1[v2];
[3:v]scale=1720:968:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0xF4F7F8,zoompan=z='min(zoom+0.00010,1.018)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=180:s=1920x1080:fps=30,setsar=1[v3];
[4:v]scale=1720:968:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0xF4F7F8,zoompan=z='min(zoom+0.00010,1.018)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=180:s=1920x1080:fps=30,setsar=1[v4];
[5:v]scale=1920:1080:force_original_aspect_ratio=increase,crop=1920:1080,zoompan=z='1.022-min(on*0.00018,0.022)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=120:s=1920x1080:fps=30,setsar=1[v5];
[v0][v1][v2][v3][v4][v5]concat=n=6:v=1:a=0,ass='$($subtitlePath.Replace('\', '/').Replace(':', '\:'))'[video]
"@ -replace "`r?`n", ""

$videoArgs = @(
    "-y",
    "-framerate", "30", "-i", $presenter,
    "-framerate", "30", "-i", $idle,
    "-framerate", "30", "-i", $recording,
    "-framerate", "30", "-i", $summary,
    "-framerate", "30", "-i", $manual,
    "-framerate", "30", "-i", $presenter,
    "-filter_complex", $filter,
    "-map", "[video]",
    "-t", "30",
    "-c:v", "libx264", "-preset", "medium", "-crf", "20",
    "-pix_fmt", "yuv420p", "-movflags", "+faststart",
    $silentVideoPath
)

& ffmpeg @videoArgs
if ($LASTEXITCODE -ne 0) {
    throw "Failed to render promo video."
}

$musicExpression = "0.018*(sin(2*PI*220*t)+0.72*sin(2*PI*277.18*t)+0.48*sin(2*PI*329.63*t))*(0.72+0.28*sin(2*PI*0.10*t))"
& ffmpeg -y -f lavfi -i "aevalsrc=$musicExpression`:d=30:s=48000" -af "lowpass=f=1400,afade=t=in:d=1.2,afade=t=out:st=28:d=2" -c:a pcm_s16le $musicPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to render promo music."
}

$audioFilter = "[1:a]volume=1.0,apad,atrim=0:30[voice];[2:a]volume=0.45[music];[voice][music]amix=inputs=2:duration=longest:dropout_transition=1,alimiter=limit=0.95[audio]"
& ffmpeg -y -i $silentVideoPath -i $voicePath -i $musicPath -filter_complex $audioFilter -map 0:v -map "[audio]" -t 30 -c:v copy -c:a aac -b:a 192k -movflags +faststart $output
if ($LASTEXITCODE -ne 0) {
    throw "Failed to combine promo video and audio."
}

Write-Host "Created $output"
