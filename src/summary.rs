use crate::datetime::LocalDateTime;
use crate::store::{Settings, Store, Task};
use reqwest::blocking::Client;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;

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
        match call_llm(&prompt, settings) {
            Ok(text) if !text.trim().is_empty() => (text, true, "已使用大模型生成".to_string()),
            Ok(_) => (
                build_local_summary(tasks, start, end, Some("大模型返回为空")),
                false,
                "大模型返回为空，已生成本地草稿".to_string(),
            ),
            Err(err) => (
                build_local_summary(tasks, start, end, Some("大模型调用失败，已生成本地草稿")),
                false,
                format!("大模型调用失败，已生成本地草稿：{}", err),
            ),
        }
    } else {
        (
            build_local_summary(
                tasks,
                start,
                end,
                Some("尚未配置大模型 API URL 和模型名"),
            ),
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

pub fn call_llm(prompt: &str, settings: &Settings) -> Result<String, String> {
    let url = normalize_chat_completions_url(settings.llm_api_url.trim());
    if url.is_empty() {
        return Err("API URL 未配置".to_string());
    }
    let model = settings.llm_model.trim();
    if model.is_empty() {
        return Err("模型名未配置".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{}", e))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    if !settings.llm_api_key.trim().is_empty() {
        let auth = format!("Bearer {}", settings.llm_api_key.trim());
        headers.insert("Authorization", auth.parse().unwrap());
    }

    let body = json!({
        "model": model,
        "temperature": 0.3,
        "messages": [
            {"role": "system", "content": "你是一个专业、克制、准确的中文工作总结助手。"},
            {"role": "user", "content": prompt}
        ]
    });

    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP 请求失败：{}", e))?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().unwrap_or_default();
        return Err(format!("API 返回错误状态 {}：{}", status, text));
    }

    let json: serde_json::Value = response
        .json()
        .map_err(|e| format!("解析 JSON 失败：{}", e))?;

    // 尝试提取 OpenAI 标准格式
    if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
        if let Some(first) = choices.first() {
            if let Some(msg) = first.get("message") {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    return Ok(content.to_string());
                }
            }
        }
    }
    // 备用兼容格式（如某些代理或本地模型）
    if let Some(text) = json.get("output_text").and_then(|v| v.as_str()) {
        return Ok(text.to_string());
    }

    Err("无法从响应中提取文本内容".to_string())
}

fn normalize_chat_completions_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{}/chat/completions", trimmed)
    }
}