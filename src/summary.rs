use crate::datetime::LocalDateTime;
use crate::store::{Settings, Store, Task};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct SummaryResult {
    pub text: String,
    pub path: PathBuf,
    pub used_llm: bool,
    pub note: String,
    pub date: String,
}

pub fn generate_and_store_summary(
    store: &Store,
    settings: &Settings,
    tasks: &[Task],
    start: LocalDateTime,
    end: LocalDateTime,
) -> Result<SummaryResult, String> {
    let prompt = build_prompt(tasks, start, end);
    let llm_configured =
        !settings.llm_api_url.trim().is_empty() && !settings.llm_model.trim().is_empty();
    let (text, used_llm, note) = if llm_configured {
        match call_llm(&prompt, settings, &store.data_dir) {
            Ok(text) if !text.trim().is_empty() => (text, true, "已使用大模型生成".to_string()),
            Ok(_) => (
                build_local_summary(tasks, start, end, Some("大模型返回为空")),
                false,
                "大模型返回为空，已生成本地草稿".to_string(),
            ),
            Err(err) => (
                build_local_summary(tasks, start, end, Some(&err)),
                false,
                format!("大模型调用失败，已生成本地草稿：{}", err),
            ),
        }
    } else {
        (
            build_local_summary(tasks, start, end, Some("尚未配置大模型 API URL 和模型名")),
            false,
            "尚未配置大模型，已生成本地草稿".to_string(),
        )
    };

    let date = end.date_string();
    let path = store
        .write_summary(&date, &text)
        .map_err(|err| format!("写入周报失败：{}", err))?;

    Ok(SummaryResult {
        text,
        path,
        used_llm,
        note,
        date,
    })
}

fn build_prompt(tasks: &[Task], start: LocalDateTime, end: LocalDateTime) -> String {
    let mut prompt = String::new();
    prompt.push_str("请根据以下完成事项生成一份简洁、专业的中文工作总结。\n");
    prompt.push_str("要求：包含本周完成、关键产出、风险/阻塞、下周建议四部分；不要编造不存在的事实；语气适合发给主管或团队。\n\n");
    prompt.push_str(&format!(
        "统计范围：{}（{}）到 {}（{}）\n",
        start.storage_string(),
        start.cn_weekday(),
        end.storage_string(),
        end.cn_weekday()
    ));
    prompt.push_str(&format!("完成事项数量：{}\n\n", tasks.len()));
    for task in tasks {
        let completed_at = task
            .completed_at
            .map(|dt| dt.storage_string())
            .unwrap_or_else(|| "未记录完成时间".to_string());
        prompt.push_str(&format!(
            "- 完成时间：{}；事项：{}；截止：{}；备注：{}\n",
            completed_at,
            task.title,
            task.due.storage_string(),
            if task.note.trim().is_empty() {
                "无"
            } else {
                task.note.trim()
            }
        ));
    }
    prompt
}

fn build_local_summary(
    tasks: &[Task],
    start: LocalDateTime,
    end: LocalDateTime,
    note: Option<&str>,
) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "# 工作总结（{} 至 {}）\n\n",
        start.storage_string(),
        end.storage_string()
    ));
    if let Some(note) = note {
        text.push_str(&format!("> 说明：{}。\n\n", note));
    }
    text.push_str("## 本周完成\n\n");
    if tasks.is_empty() {
        text.push_str("- 本周期暂无已完成事项记录。\n");
    } else {
        for task in tasks {
            let completed_at = task
                .completed_at
                .map(|dt| dt.short_string())
                .unwrap_or_else(|| "未记录".to_string());
            text.push_str(&format!(
                "- {}：{}（截止 {}）",
                completed_at,
                task.title,
                task.due.short_string()
            ));
            if !task.note.trim().is_empty() {
                text.push_str(&format!("；备注：{}", task.note.trim()));
            }
            text.push('\n');
        }
    }
    text.push_str("\n## 关键产出\n\n");
    text.push_str(&format!("- 共完成 {} 项工作。\n", tasks.len()));
    text.push_str("\n## 风险/阻塞\n\n- 请根据实际情况补充延期、协作或资源风险。\n");
    text.push_str(
        "\n## 下周建议\n\n- 优先处理临近截止时间的事项，并继续补充任务备注，便于下次自动总结。\n",
    );
    text
}

fn call_llm(prompt: &str, settings: &Settings, data_dir: &PathBuf) -> Result<String, String> {
    fs::create_dir_all(data_dir).map_err(|err| format!("准备大模型临时目录失败：{}", err))?;
    let stamp = LocalDateTime::now()
        .storage_string()
        .replace([' ', ':', '-'], "_");
    let prompt_path = data_dir.join(format!("llm-prompt-{}.txt", stamp));
    let out_path = data_dir.join(format!("llm-output-{}.txt", stamp));
    let script_path = data_dir.join("call-llm.ps1");

    fs::write(&prompt_path, prompt).map_err(|err| format!("写入提示词失败：{}", err))?;
    fs::write(&script_path, POWERSHELL_SCRIPT)
        .map_err(|err| format!("写入大模型调用脚本失败：{}", err))?;

    let output = Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg("-ApiUrl")
        .arg(settings.llm_api_url.trim())
        .arg("-Model")
        .arg(settings.llm_model.trim())
        .arg("-PromptFile")
        .arg(&prompt_path)
        .arg("-OutFile")
        .arg(&out_path)
        .env("RUST_TODO_WIDGET_LLM_KEY", settings.llm_api_key.trim())
        .output()
        .map_err(|err| format!("启动 PowerShell 调用大模型失败：{}", err))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            "大模型命令返回失败状态".to_string()
        } else {
            detail
        });
    }

    fs::read_to_string(&out_path).map_err(|err| format!("读取大模型输出失败：{}", err))
}

const POWERSHELL_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory = $true)][string]$ApiUrl,
    [Parameter(Mandatory = $true)][string]$Model,
    [Parameter(Mandatory = $true)][string]$PromptFile,
    [Parameter(Mandatory = $true)][string]$OutFile
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$prompt = Get-Content -Raw -Encoding UTF8 $PromptFile
$body = @{
    model = $Model
    temperature = 0.3
    messages = @(
        @{ role = "system"; content = "你是一个专业、克制、准确的中文工作总结助手。" },
        @{ role = "user"; content = $prompt }
    )
} | ConvertTo-Json -Depth 12

$headers = @{ "Content-Type" = "application/json" }
if ($env:RUST_TODO_WIDGET_LLM_KEY) {
    $headers["Authorization"] = "Bearer $env:RUST_TODO_WIDGET_LLM_KEY"
}

$response = Invoke-RestMethod -Uri $ApiUrl -Method Post -Headers $headers -Body $body
$text = $null
if ($response.choices -and $response.choices.Count -gt 0) {
    $text = $response.choices[0].message.content
}
if (-not $text -and $response.output_text) {
    $text = $response.output_text
}
if (-not $text) {
    throw "接口响应中没有可识别的文本内容"
}

Set-Content -Path $OutFile -Value $text -Encoding UTF8
"#;
