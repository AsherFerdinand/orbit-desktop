param([ValidateSet('dev','build','check')][string]$Task = 'dev')
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$localTools = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\work\toolchain'))
if (Test-Path (Join-Path $localTools 'rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe')) {
    $env:CARGO_INCREMENTAL = '0'
    $env:RUSTUP_HOME = Join-Path $localTools 'rustup'
    $env:CARGO_HOME = Join-Path $localTools 'cargo'
    $env:PATH = (Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-msvc\bin') + ';' + $env:PATH
}
function Invoke-Checked([scriptblock]$Command) { & $Command; if ($LASTEXITCODE -ne 0) { throw "Command failed with exit code $LASTEXITCODE" } }
switch ($Task) {
    'dev' { Invoke-Checked { npm.cmd run tauri -- dev } }
    'build' { Invoke-Checked { npm.cmd run tauri -- build --no-bundle } }
    'check' {
        Invoke-Checked { npm.cmd run build }
        Invoke-Checked { npm.cmd test }
        Invoke-Checked { cargo fmt --manifest-path src-tauri\Cargo.toml -- --check }
        Invoke-Checked { cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings }
        Invoke-Checked { cargo test --manifest-path src-tauri\Cargo.toml }
    }
}
