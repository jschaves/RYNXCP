#!/usr/bin/env pwsh
# End-to-end smoke test in simulation mode. Requires: cargo build --workspace
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

$dataDir = Join-Path $root 'data'
if (Test-Path $dataDir) { Remove-Item -Recurse -Force $dataDir }
Get-Process rynxd, rynx-api -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 400

$logs = Join-Path $env:TEMP 'rynx-smoke'
if (Test-Path $logs) { Remove-Item -Recurse -Force $logs }
New-Item -ItemType Directory -Force -Path $logs | Out-Null

function Start-RynxProcess([string]$exe, [string]$name) {
    $out = Join-Path $logs "$name.out"
    $err = Join-Path $logs "$name.err"
    $process = Start-Process -FilePath (Join-Path $root $exe) `
        -ArgumentList '--config', "$name.toml" `
        -WorkingDirectory $root `
        -RedirectStandardOutput $out `
        -RedirectStandardError $err `
        -PassThru
    Start-Sleep -Milliseconds 1500
    if ($process.HasExited) {
        Write-Host "--- $name stderr ---"
        Get-Content $err -ErrorAction SilentlyContinue
        throw "$name exited unexpectedly"
    }
    return $process
}

$daemon = Start-RynxProcess 'target\debug\rynxd.exe' 'rynxd'
$api = $null
try {
    & (Join-Path $root 'target\debug\rynxctl.exe') --config rynx-api.toml admin create admin `
        --email admin@example.com --password 'Sup3r-Secret-Pass' --role owner | Out-Null

    $api = Start-RynxProcess 'target\debug\rynx-api.exe' 'rynx-api'

    $base = 'http://127.0.0.1:8080'
    $login = Invoke-RestMethod -Uri "$base/api/v1/auth/login" -Method Post `
        -ContentType 'application/json' `
        -Body (@{ username = 'admin'; password = 'Sup3r-Secret-Pass' } | ConvertTo-Json) `
        -SessionVariable sess
    if (-not $login.csrf_token) { throw 'login did not return a CSRF token' }
    $headers = @{ 'x-csrf-token' = $login.csrf_token }

    function PostJson([string]$path, $body) {
        Invoke-RestMethod -Uri "$base$path" -Method Post -WebSession $sess -Headers $headers `
            -ContentType 'application/json' -Body ($body | ConvertTo-Json -Depth 12)
    }

    function RequestJson([string]$method, [string]$path, $body = $null) {
        $params = @{ Uri = "$base$path"; Method = $method; WebSession = $sess; Headers = $headers }
        if ($null -ne $body) {
            $params.ContentType = 'application/json'
            $params.Body = ($body | ConvertTo-Json -Depth 12)
        }
        Invoke-RestMethod @params
    }

    function ExpectStatus([int]$expected, [scriptblock]$action) {
        try {
            & $action | Out-Null
            throw "expected HTTP $expected but request succeeded"
        } catch {
            $response = $_.Exception.Response
            $actual = if ($null -eq $response) { 0 } else { [int]$response.StatusCode }
            if ($actual -ne $expected) {
                throw "expected HTTP $expected, got ${actual}: $($_.Exception.Message)"
            }
        }
    }

    $page = Invoke-WebRequest -Uri "$base/" -UseBasicParsing
    if ($page.StatusCode -ne 200 -or $page.Content -notmatch 'RYNXCP') { throw 'web UI was not served' }
    $scriptJs = Invoke-WebRequest -Uri "$base/app.js" -UseBasicParsing
    if ($scriptJs.StatusCode -ne 200) { throw 'app.js was not served' }

    Write-Host '==> accounts, apps, websites'
    $account = PostJson '/api/v1/accounts' @{
        username = 'acme'; email = 'acme@example.com'; disk_limit_mb = 2048
        php_version = '8.3'; enabled = $true
    }
    if ($account.kind -ne 'account') { throw 'unexpected account response' }

    $app = PostJson '/api/v1/apps' @{
        username = 'acme'; name = 'api'; runtime = @{ engine = 'node'; version = '22' }
        port = 4100; entrypoint = 'server.js'; command = 'node'
        args = @('server.js'); env = @{ NODE_ENV = 'production' }; enabled = $true
    }
    if ($app.kind -ne 'app') { throw 'unexpected app response' }

    $site = PostJson '/api/v1/websites' @{
        domain = 'example.com'; username = 'acme'; aliases = @('www.example.com')
        runtime = @{ engine = 'node'; version = '22' }; app = 'api'
        docroot = 'public_html'; tls = 'disabled'; enabled = $true
    }
    if ($site.kind -ne 'website') { throw 'unexpected website response' }

    $siteConf = Join-Path $root 'data/daemon/sandbox/nginx/sites-available/example.com.conf'
    if (-not (Test-Path $siteConf)) { throw 'nginx site was not rendered' }
    if (-not (Select-String -Path $siteConf -Pattern 'proxy_pass http://127.0.0.1:4100' -Quiet)) {
        throw 'node site does not proxy to the application port'
    }

    Write-Host '==> php pool auto-provisioning'
    $phpSite = PostJson '/api/v1/websites' @{
        domain = 'php.example.com'; username = 'acme'; aliases = @()
        runtime = @{ engine = 'php'; version = '8.3' }; app = $null
        docroot = 'public_html'; tls = 'disabled'; enabled = $true
    }
    if ($phpSite.kind -ne 'website') { throw 'unexpected php website response' }
    $phpConf = Join-Path $root 'data/daemon/sandbox/nginx/sites-available/php.example.com.conf'
    if (-not (Select-String -Path $phpConf -Pattern 'fastcgi_pass unix:' -Quiet)) {
        throw 'php site does not use fastcgi'
    }
    $poolConf = Join-Path $root 'data/daemon/sandbox/php/8.3/fpm/pool.d/rynx-acme.conf'
    if (-not (Test-Path $poolConf)) { throw 'php pool was not rendered' }

    Write-Host '==> account suspension/resume'
    $suspended = PostJson '/api/v1/accounts' @{
        username = 'acme'; email = 'acme@example.com'; disk_limit_mb = 2048
        php_version = '8.3'; enabled = $false
    }
    if ($suspended.account.spec.enabled) { throw 'account was not suspended' }
    if (Test-Path $siteConf) { throw 'suspended account still has an enabled vhost' }
    $resumed = PostJson '/api/v1/accounts' @{
        username = 'acme'; email = 'acme@example.com'; disk_limit_mb = 2048
        php_version = '8.3'; enabled = $true
    }
    if (-not $resumed.account.spec.enabled) { throw 'account was not resumed' }
    if (-not (Test-Path $siteConf)) { throw 'resumed account vhost was not restored' }

    Write-Host '==> dns'
    $dns = RequestJson 'GET' '/api/v1/dns'
    if ($dns.kind -ne 'dns_zones' -or $dns.zones.Count -lt 2) { throw 'expected DNS zones for both websites' }
    $zone = $dns.zones | Where-Object { $_.domain -eq 'example.com' } | Select-Object -First 1
    if (-not ($zone.records | Where-Object { $_.spec.name -eq '_dmarc' })) { throw 'DMARC record missing' }
    if (-not ($zone.records | Where-Object { $_.spec.name -like '*._domainkey' })) { throw 'DKIM record missing' }
    $zoneFile = Join-Path $root 'data/daemon/sandbox/bind/zones/example.com.db'
    if (-not (Select-String -Path $zoneFile -Pattern 'v=DMARC1' -Quiet)) { throw 'DMARC not rendered in zone file' }

    $record = RequestJson 'POST' '/api/v1/dns/example.com/records' @{
        domain = 'example.com'; name = 'status'; rtype = 'A'; value = '203.0.113.55'; ttl = 300; priority = $null
    }
    $newRecord = $record.zone.records | Where-Object { $_.spec.name -eq 'status' } | Select-Object -First 1
    if (-not $newRecord) { throw 'manual DNS record was not added' }
    $null = RequestJson 'DELETE' "/api/v1/dns/example.com/records/$($newRecord.id)"
    $null = RequestJson 'POST' '/api/v1/dns/example.com/mail'

    Write-Host '==> databases and ftp'
    $database = RequestJson 'POST' '/api/v1/databases' @{ domain = 'example.com' }
    if ($database.kind -ne 'database') { throw 'database creation failed' }
    if ($database.database.username -notlike 'rynx_*') { throw 'unexpected database name' }
    if ((RequestJson 'GET' '/api/v1/databases').databases.Count -ne 1) { throw 'expected one database' }

    $ftp = RequestJson 'POST' '/api/v1/ftp' @{ domain = 'example.com'; username = 'deploy'; readonly = $false }
    if ($ftp.kind -ne 'ftp') { throw 'ftp account creation failed' }
    if ($ftp.account.password.Length -lt 12) { throw 'ftp password is too short' }
    if ((RequestJson 'GET' '/api/v1/ftp').accounts.Count -ne 1) { throw 'expected one ftp account' }

    Write-Host '==> backups'
    $backup = RequestJson 'POST' '/api/v1/backups' @{ username = 'acme' }
    if ($backup.kind -ne 'backup') { throw 'backup creation failed' }
    if (-not (Test-Path $backup.backup.path)) { throw 'backup file does not exist' }
    if ((RequestJson 'POST' "/api/v1/backups/$($backup.backup.id)/restore").kind -ne 'ack') { throw 'backup restore failed' }

    Write-Host '==> certificates'
    $secureSite = PostJson '/api/v1/websites' @{
        domain = 'secure.example.com'; username = 'acme'; aliases = @('www.secure.example.com')
        runtime = @{ engine = 'static' }; app = $null
        docroot = 'public_html'; tls = 'lets_encrypt'; enabled = $true
    }
    if ($secureSite.kind -ne 'website') { throw 'secure website creation failed' }
    $cert = RequestJson 'POST' '/api/v1/certs' @{ domain = 'secure.example.com'; email = 'admin@example.com'; staging = $false }
    if ($cert.kind -ne 'cert') { throw 'certificate issue failed' }
    $secureConf = Join-Path $root 'data/daemon/sandbox/nginx/sites-available/secure.example.com.conf'
    if (-not (Select-String -Path $secureConf -Pattern 'listen 443 ssl' -Quiet)) { throw 'vhost was not upgraded to TLS' }
    if ((RequestJson 'POST' '/api/v1/certs/renew').kind -ne 'ack') { throw 'certificate renew failed' }

    Write-Host '==> monitor, programs, console'
    $services = RequestJson 'GET' '/api/v1/monitor/services'
    if ($services.kind -ne 'service_stats' -or $services.services.Count -lt 2) { throw 'service stats missing' }
    if ((RequestJson 'GET' '/api/v1/monitor/history?minutes=1440').kind -ne 'metrics_history') { throw 'metrics history missing' }
    $programs = RequestJson 'GET' '/api/v1/system/programs'
    if ($programs.kind -ne 'programs' -or $programs.programs.Count -lt 5) { throw 'programs list missing' }
    if ((RequestJson 'GET' '/api/v1/system/updates').kind -ne 'updates') { throw 'updates list missing' }
    $console = RequestJson 'POST' '/api/v1/console' @{ action = @{ action = 'disk_usage' } }
    if ($console.kind -ne 'console_output' -or -not $console.output) { throw 'console action failed' }

    Write-Host '==> firewall'
    if ((RequestJson 'GET' '/api/v1/firewall').kind -ne 'firewall') { throw 'firewall status missing' }
    $null = RequestJson 'POST' '/api/v1/firewall/rules' @{ action = 'allow'; proto = 'tcp'; port = 8443; source = '203.0.113.0/24'; comment = 'test' }
    $fw = RequestJson 'POST' '/api/v1/firewall/block' @{ ip = '198.51.100.7'; seconds = 3600; reason = 'test' }
    if (-not ($fw.state.blocked | Where-Object { $_.ip -eq '198.51.100.7' })) { throw 'blocked IP not stored' }
    $fw = RequestJson 'POST' '/api/v1/firewall/whitelist' @{ ip = '198.51.100.8' }
    if (-not ($fw.state.whitelist -contains '198.51.100.8')) { throw 'whitelist not stored' }
    $null = RequestJson 'DELETE' '/api/v1/firewall/block/198.51.100.7'
    $null = RequestJson 'DELETE' '/api/v1/firewall/whitelist/198.51.100.8'

    Write-Host '==> security and impersonation'
    $stats = RequestJson 'GET' '/api/v1/security/stats?hours=24'
    if ($null -eq $stats.failures) { throw 'security stats missing' }
    $events = RequestJson 'GET' '/api/v1/security/auth-events?limit=50'
    if ($events.events.Count -lt 1) { throw 'auth events missing' }

    $viewer = PostJson '/api/v1/users' @{
        username = 'viewer1'; email = 'viewer1@example.com'; password = 'Viewer-Strong-Pass3'; role = 'viewer'
    }
    if ($viewer.role -ne 'viewer') { throw 'viewer creation failed' }
    $impersonated = RequestJson 'POST' '/api/v1/users/viewer1/impersonate'
    if ($impersonated.impersonated_by -ne 'admin' -or $impersonated.user.username -ne 'viewer1') {
        throw 'impersonation response is invalid'
    }
    $headers['x-csrf-token'] = $impersonated.csrf_token
    $me = RequestJson 'GET' '/api/v1/auth/me'
    if ($me.user.username -ne 'viewer1' -or $me.impersonated_by -ne 'admin') { throw 'impersonated session is invalid' }
    $restored = RequestJson 'POST' '/api/v1/auth/stop-impersonation'
    if ($restored.user.username -ne 'admin' -or $restored.impersonated_by) { throw 'stop impersonation failed' }
    $headers['x-csrf-token'] = $restored.csrf_token

    Write-Host '==> security checks'
    ExpectStatus 400 {
        PostJson '/api/v1/accounts' @{
            username = 'root'; email = 'root@example.com'; disk_limit_mb = 1024
            php_version = $null; enabled = $true
        }
    }
    ExpectStatus 409 {
        RequestJson 'POST' '/api/v1/databases' @{ domain = 'example.com' }
    }
    ExpectStatus 403 {
        $login2 = Invoke-RestMethod -Uri "$base/api/v1/auth/login" -Method Post `
            -ContentType 'application/json' `
            -Body (@{ username = 'admin'; password = 'Sup3r-Secret-Pass' } | ConvertTo-Json) `
            -SessionVariable fresh
        $null = $login2.csrf_token
        Invoke-RestMethod -Uri "$base/api/v1/accounts" -Method Post -WebSession $fresh `
            -ContentType 'application/json' `
            -Body (@{ username = 'nocsrf'; email = 'n@example.com'; disk_limit_mb = 1024; php_version = $null; enabled = $true } | ConvertTo-Json)
    }
    ExpectStatus 401 {
        Invoke-RestMethod -Uri "$base/api/v1/auth/login" -Method Post -ContentType 'application/json' `
            -Body (@{ username = 'admin'; password = 'wrong-password' } | ConvertTo-Json)
    }
    $anon = 0
    try {
        Invoke-RestMethod -Uri "$base/api/v1/accounts" | Out-Null
        $anon = 200
    } catch {
        $anon = [int]$_.Exception.Response.StatusCode
    }
    if ($anon -ne 401) { throw "unauthenticated access returned $anon" }

    Write-Host '==> audit and reconcile'
    $audit = RequestJson 'GET' '/api/v1/audit?limit=50'
    if ($audit.entries.Count -lt 8) { throw 'audit log is missing events' }
    if ($audit.entries[-1].hash.Length -ne 64) { throw 'audit hashes are malformed' }
    if ((RequestJson 'POST' '/api/v1/reconcile').kind -ne 'ack') { throw 'reconcile did not return an ack' }

    Write-Host '==> cleanup'
    $null = RequestJson 'DELETE' "/api/v1/ftp/$($ftp.account.id)"
    $null = RequestJson 'DELETE' "/api/v1/databases/$($database.database.id)"
    $null = RequestJson 'DELETE' "/api/v1/backups/$($backup.backup.id)"

    Write-Host ''
    Write-Host 'SMOKE OK: auth, RBAC, CSRF, accounts, runtimes, DNS, DB, FTP, backups, TLS, monitor, console, firewall, impersonation, audit, reconcile.'
} finally {
    if ($api -and -not $api.HasExited) { Stop-Process -Id $api.Id -Force }
    if ($daemon -and -not $daemon.HasExited) { Stop-Process -Id $daemon.Id -Force }
}


