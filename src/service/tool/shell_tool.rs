// Shell 执行工具
use tokio::process::Command;

use super::ToolResult;

// PowerShell UTF-8 配置内容
const POWERSHELL_UTF8_SETTING: &str = r#"# UTF-8 encoding setting
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()

# Basic file operation commands UTF-8 encoding
$PSDefaultParameterValues['Out-File:Encoding'] = 'utf8'
$PSDefaultParameterValues['Set-Content:Encoding'] = 'utf8'
$PSDefaultParameterValues['Add-Content:Encoding'] = 'utf8'
"#;

// Windows 下配置 PowerShell UTF-8 编码
pub async fn setup_powershell_utf8() {
    if !cfg!(windows) {
        return;
    }

    let home_dir = crate::config::home_dir();
    // 依次配置 Windows PowerShell 5.1 与 PowerShell 7 的输出编码
    for profile_dir in ["WindowsPowerShell", "PowerShell"] {
        let profile_path = std::path::Path::new(&home_dir)
            .join("Documents")
            .join(profile_dir)
            .join("Microsoft.PowerShell_profile.ps1");
        if let Some(parent) = profile_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&profile_path, POWERSHELL_UTF8_SETTING).await;
    }

    tracing::info!("PowerShell default encoding configured");
}

// 执行 shell 命令
pub async fn execute(cmd_and_args: &[String], work_dir: &str) -> ToolResult {
    if cmd_and_args.is_empty() {
        return ("No command provided".to_string(), true);
    }

    let mut cmd = Command::new(&cmd_and_args[0]);
    for arg in &cmd_and_args[1..] {
        cmd.arg(arg);
    }
    cmd.current_dir(work_dir);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    // Windows 下隐藏子进程控制台窗口, 避免桌面模式调用外部模型时弹出黑框
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }

    match cmd.output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let tool_content = format!("{}{}", stdout, stderr);
            let is_error = !output.status.success();
            (tool_content, is_error)
        }
        Err(e) => (format!("Failed to execute command: {}", e), true),
    }
}
