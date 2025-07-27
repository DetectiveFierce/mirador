# Mirador Release Script for Windows
# Usage: .\scripts\create_release.ps1 <version> <platform>

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,
    
    [Parameter(Mandatory=$true)]
    [string]$Platform
)

# Error handling
$ErrorActionPreference = "Stop"

# Validate parameters
if ([string]::IsNullOrEmpty($Version) -or [string]::IsNullOrEmpty($Platform)) {
    Write-Host "Usage: .\scripts\create_release.ps1 <version> <platform>"
    Write-Host "Example: .\scripts\create_release.ps1 v0.0.1a Windows"
    exit 1
}

# Create releases directory and version subfolder if they don't exist
if (!(Test-Path "releases")) {
    New-Item -ItemType Directory -Path "releases" | Out-Null
}

$VersionFolder = "releases\$Version"
if (!(Test-Path $VersionFolder)) {
    New-Item -ItemType Directory -Path $VersionFolder | Out-Null
}

# Build the project
Write-Host "Building Mirador..."
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

# Copy binary with proper naming
$BinaryName = "Mirador-${Version}-${Platform}.exe"
$SourcePath = "target\release\mirador.exe"
$DestPath = "$VersionFolder\$BinaryName"

if (!(Test-Path $SourcePath)) {
    Write-Error "Executable not found at $SourcePath"
    exit 1
}

Copy-Item $SourcePath $DestPath

Write-Host "Release created: $VersionFolder\$BinaryName"

# Get file size
$FileSize = (Get-Item $DestPath).Length
$FileSizeMB = [math]::Round($FileSize / 1MB, 2)
Write-Host "Binary size: $FileSizeMB MB"

# Create checksum
$Hash = Get-FileHash -Path $DestPath -Algorithm SHA256
$Hash.Hash | Out-File -FilePath "$VersionFolder\$BinaryName.sha256" -Encoding ASCII

Write-Host "Checksum created: $VersionFolder\$BinaryName.sha256"
Write-Host "Hash: $($Hash.Hash)" 