param([switch]$NoBrowser)
$ErrorActionPreference = 'Stop'
$databaseUrl = 'http://127.0.0.1:18766/v1/database/stronghold-local/identity'
$databaseReady = $false
try {
    $databaseResponse = Invoke-WebRequest -Uri $databaseUrl -UseBasicParsing -TimeoutSec 2
    $databaseReady = $databaseResponse.StatusCode -eq 200
} catch {}
if (-not $databaseReady) {
    $dataDirectory = Join-Path $PSScriptRoot 'server\.spacetime-local'
    $serverExe = Join-Path $PSScriptRoot 'runtime\spacetimedb-standalone.exe'
    $serverArgs = @('start', '--listen-addr', '127.0.0.1:18766', '--data-dir',
        ('"' + $dataDirectory + '"'), '--jwt-pub-key-path',
        ('"' + (Join-Path $dataDirectory 'jwt-public.pem') + '"'), '--jwt-priv-key-path',
        ('"' + (Join-Path $dataDirectory 'jwt-private.pem') + '"'))
    Start-Process -FilePath $serverExe -ArgumentList $serverArgs -WindowStyle Hidden
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        Start-Sleep -Milliseconds 250
        try {
            $databaseResponse = Invoke-WebRequest -Uri $databaseUrl -UseBasicParsing -TimeoutSec 1
            if ($databaseResponse.StatusCode -eq 200) { $databaseReady = $true; break }
        } catch {}
    }
}
if (-not $databaseReady) { throw 'Die lokale Spielwelt konnte nicht gestartet werden.' }
$gameUrl = 'http://127.0.0.1:18765/'
$running = $false
try {
    $response = Invoke-WebRequest -Uri $gameUrl -UseBasicParsing -TimeoutSec 2
    $running = $response.Content -match 'Stronghold'
} catch {}
if (-not $running) {
    $pythonCommand = (Get-Command python -ErrorAction Stop).Source
    Start-Process -FilePath $pythonCommand -ArgumentList @('"' + (Join-Path $PSScriptRoot 'serve.py') + '"') -WindowStyle Hidden
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        Start-Sleep -Milliseconds 200
        try {
            $response = Invoke-WebRequest -Uri $gameUrl -UseBasicParsing -TimeoutSec 1
            if ($response.Content -match 'Stronghold') { $running = $true; break }
        } catch {}
    }
}
if (-not $running) { throw 'Der lokale Spielserver konnte nicht gestartet werden.' }
if (-not $NoBrowser) { Start-Process $gameUrl }
