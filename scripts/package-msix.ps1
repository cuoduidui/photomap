# PhotoMap MSIX 打包脚本（微软商店上架用）
# 依赖：Windows SDK（提供 MakeAppx.exe / signtool.exe）
# 用法：powershell -ExecutionPolicy Bypass -File scripts/package-msix.ps1
#
# 注意：正式提交商店前，请把下面占位符替换为 Partner Center 中的真实值：
#   $PackageName = Partner Center 保留名称对应的 Package Identity Name
#   $PublisherDN = Partner Center 开发者账号的发布者显示名（带 CN= 前缀）
# 签名证书可选用 cert.pfx（商店会用自己的证书替换签名，未签名包也能提交）

$ErrorActionPreference = "Stop"

# ===== 占位符，请修改 =====
$PackageName  = "PhotoMap"
$PublisherDN  = "CN=YourPublisherName"
$Version      = "1.0.0.0"
$Arch         = "x64"
$OutputDir    = Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\msix"
$WorkDir      = Join-Path $env:TEMP "photomap-msix-work"

$AppExe       = "PhotoMap.exe"

# ===== 0. 先构建 MSI（若已存在可跳过） =====
$msiDir = Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\msi"
$msiFound = Get-ChildItem $msiDir -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $msiFound) {
  Write-Host "未找到 MSI，开始构建（首次需数分钟）..."
  Push-Location (Join-Path $PSScriptRoot "..")
  try { npm run tauri build -- --bundles msi } finally { Pop-Location }
  $msiFound = Get-ChildItem $msiDir -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1
}
if (-not $msiFound) { throw "MSI 构建失败，请检查 tauri build 输出。" }

# ===== 1. 找到 Windows SDK 工具 =====
$makeappx = Get-ChildItem "$env:ProgramFiles(x86)\Windows Kits\10\bin" -Recurse -Filter MakeAppx.exe -ErrorAction SilentlyContinue |
  Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) {
  throw "未找到 MakeAppx.exe，请先安装 Windows SDK（Windows 10 SDK）。"
}

# ===== 2. 准备打包目录 =====
if (Test-Path $WorkDir) { Remove-Item $WorkDir -Recurse -Force }
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

$installDir = Join-Path $WorkDir "Install"
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $WorkDir "assets") -Force | Out-Null

# 复制 release 目录中的安装产物
$releaseDir = Join-Path $PSScriptRoot "..\src-tauri\target\release"
Get-ChildItem $releaseDir -Filter "*.exe" -ErrorAction SilentlyContinue | Copy-Item -Destination $installDir
Get-ChildItem $releaseDir -Filter "*.dll" -ErrorAction SilentlyContinue | Copy-Item -Destination $installDir
if (Test-Path (Join-Path $releaseDir "resources")) {
  Copy-Item (Join-Path $releaseDir "resources") -Destination $installDir -Recurse
}

Copy-Item (Join-Path $PSScriptRoot "..\src-tauri\icons\512x512.png") (Join-Path $WorkDir "assets\512x512.png")
Copy-Item (Join-Path $PSScriptRoot "..\src-tauri\icons\icon.ico") (Join-Path $WorkDir "assets\icon.ico")

# ===== 3. 生成 AppxManifest.xml（命名空间用拼接避免编辑器/终端转义问题） =====
$ns1 = "http:" + "//schemas.microsoft.com/appx/manifest/foundation/windows10"
$ns2 = "http:" + "//schemas.microsoft.com/appx/manifest/uap/windows10"
$ns3 = "http:" + "//schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="$ns1"
         xmlns:uap="$ns2"
         xmlns:rescap="$ns3"
         IgnorableNamespaces="uap rescap">
  <Identity Name="$PackageName" Publisher="$PublisherDN" Version="$Version" ProcessorArchitecture="$Arch" />
  <Properties>
    <DisplayName>PhotoMap</DisplayName>
    <PublisherDisplayName>PhotoMap Dev</PublisherDisplayName>
    <Logo>assets/512x512.png</Logo>
  </Properties>
  <Resources>
    <Resource Language="en-us"/>
    <Resource Language="zh-cn"/>
    <Resource Language="ja-jp"/>
    <Resource Language="fr-fr"/>
    <Resource Language="ko-kr"/>
    <Resource Language="de-de"/>
    <Resource Language="ru-ru"/>
  </Resources>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
  <Applications>
    <Application Id="App" Executable="$AppExe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="PhotoMap" Square150x150Logo="assets/512x512.png"
                          Square44x44Logo="assets/512x512.png" Description="Interactive travel photo map"
                          BackgroundColor="transparent" />
    </Application>
  </Applications>
</Package>
"@
Set-Content -Path (Join-Path $WorkDir "AppxManifest.xml") -Value $manifest -Encoding UTF8

# ===== 4. makeappx 打包 =====
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$msixOut = Join-Path $OutputDir ("PhotoMap_" + $Version + "_" + $Arch + ".msix")
& $makeappx.FullName pack /d $WorkDir /p $msixOut /o
if ($LASTEXITCODE -ne 0) { throw "makeappx 打包失败" }

# ===== 5. 签名（有证书则签，无证书跳过——商店提交允许未签名包） =====
$signtool = Get-ChildItem "$env:ProgramFiles(x86)\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
  Sort-Object FullName -Descending | Select-Object -First 1
$certPath = Join-Path $PSScriptRoot "..\store\cert.pfx"
if ($signtool -and (Test-Path $certPath)) {
  $certPwd = Read-Host -Prompt "输入证书密码（cert.pfx）" -AsSecureString
  $ptr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($certPwd)
  try {
    $plain = [Runtime.InteropServices.Marshal]::PtrToStringAuto($ptr)
    & $signtool.FullName sign /fd SHA256 /f $certPath /p $plain $msixOut
  } finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($ptr)
  }
} else {
  Write-Host "未找到签名证书，生成未签名 MSIX（可直接提交商店，商店会重新签名）。"
}

Write-Host ""
Write-Host "MSIX 已生成：$msixOut"
Write-Host "上传到 Partner Center 前，请确认 AppxManifest 中的 Package Name / Publisher 与商店保留名称一致。"
