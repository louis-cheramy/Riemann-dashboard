# Script de compilation Windows (MSVC + Rust)
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$vcvars = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

if (-not (Test-Path (Join-Path $cargoBin "cargo.exe"))) {
    Write-Host "Rust / cargo introuvable."
    Write-Host "Installez Rust puis redemarrez le terminal :"
    Write-Host "  winget install Rustlang.Rustup"
    exit 1
}

if (-not (Test-Path $vcvars)) {
    Write-Host "Visual Studio Build Tools introuvables."
    Write-Host "Installez-les avec : winget install Microsoft.VisualStudio.2022.BuildTools"
    exit 1
}

Write-Host "Compilation de Riemann Dashboard (release)..."
$projectDir = $PSScriptRoot

# cmd n'a pas le PATH utilisateur : ajouter explicitement .cargo\bin
cmd /c "`"$vcvars`" >nul && set PATH=$cargoBin;%PATH% && cd /d `"$projectDir`" && cargo build --release --bin riemann-dashboard"

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "Succes ! Executable : target\release\riemann-dashboard.exe"
} else {
    Write-Host ""
    Write-Host "Echec. Verifiez Rust et Visual Studio Build Tools (C++)."
}
