# is02 Firmware Flash Script
# 2台のESP32にis02ファームウェアを書き込みます

$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $SCRIPT_DIR

Write-Host "====================================" -ForegroundColor Cyan
Write-Host "is02 Firmware Flash Tool" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""

# ポート指定
$PORTS = @("COM4", "COM5")

foreach ($PORT in $PORTS) {
    Write-Host "=== Flashing $PORT ===" -ForegroundColor Yellow
    Write-Host ""

    try {
        py -m esptool --chip esp32 --port $PORT --baud 460800 write-flash `
            0x1000 build\is02.ino.bootloader.bin `
            0x8000 build\is02.ino.partitions.bin `
            0x10000 build\is02.ino.bin

        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "✓ $PORT への書き込みが完了しました" -ForegroundColor Green
            Write-Host ""
        } else {
            Write-Host ""
            Write-Host "✗ $PORT への書き込みに失敗しました" -ForegroundColor Red
            Write-Host ""
        }
    } catch {
        Write-Host "✗ エラー: $_" -ForegroundColor Red
    }

    Write-Host ""
    Start-Sleep -Seconds 2
}

Write-Host "====================================" -ForegroundColor Cyan
Write-Host "書き込み処理が完了しました" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "次のステップ:" -ForegroundColor Yellow
Write-Host "1. デバイスが再起動するまで待つ（約5秒）"
Write-Host "2. シリアルモニタでIPアドレスを確認"
Write-Host "   例: py -m serial.tools.miniterm COM4 115200"
Write-Host ""
