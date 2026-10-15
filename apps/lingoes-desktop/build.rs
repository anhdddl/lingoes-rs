use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");

    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        embed_windows_resources();
    }
}

fn embed_windows_resources() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_path = out_dir.join("lingoes-desktop.manifest");
    let rc_path = out_dir.join("lingoes-desktop.rc");
    let res_path = out_dir.join("lingoes-desktop.res");

    let manifest = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="0.1.2.0"
    processorArchitecture="*"
    name="LingoesRS.LingoesDesktop"
    type="win32"
  />
  <description>Lingoes RS - Dictionary and Text Translation for Windows 11</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
      <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <!-- Windows 7 -->
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
</assembly>
"#;
    let _ = fs::write(&manifest_path, manifest);

    let manifest_escaped = manifest_path.display().to_string().replace('\\', "/");
    let ico_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("ui")
        .join("assets")
        .join("lingoes.ico");
    let ico_escaped = ico_path.display().to_string().replace('\\', "/");

    let rc_content = format!("1 ICON \"{ico_escaped}\"\n1 24 \"{manifest_escaped}\"\n");
    let _ = fs::write(&rc_path, rc_content);

    let status = Command::new("llvm-rc")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc_path)
        .status();

    if let Ok(s) = status {
        if s.success() {
            println!("cargo:rustc-link-arg={}", res_path.display());
        }
    }
}
