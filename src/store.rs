use crate::datetime::LocalDateTime;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub due: LocalDateTime,
    pub note: String,
    pub created_at: LocalDateTime,
    pub completed: bool,
    pub completed_at: Option<LocalDateTime>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub summary_weekday: u32,
    pub auto_summary: bool,
    pub launch_on_startup: bool,
    pub background_color: String,
    pub window_alpha: u8,
    pub llm_api_url: String,
    pub llm_model: String,
    pub llm_api_key: String,
    pub last_summary_date: String,
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: i32,
    pub window_height: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            summary_weekday: 5,
            auto_summary: true,
            launch_on_startup: false,
            background_color: "#F5F7FA".to_string(),
            window_alpha: 232,
            llm_api_url: String::new(),
            llm_model: String::new(),
            llm_api_key: String::new(),
            last_summary_date: String::new(),
            window_x: 80,
            window_y: 80,
            window_width: 420,
            window_height: 620,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Store {
    pub data_dir: PathBuf,
    tasks_path: PathBuf,
    settings_path: PathBuf,
    summaries_dir: PathBuf,
}

impl Store {
    pub fn new() -> io::Result<Self> {
        let base = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(PathBuf::from))
            .unwrap_or(std::env::current_dir()?);
        let data_dir = base.join("data");
        let summaries_dir = data_dir.join("summaries");
        fs::create_dir_all(&summaries_dir)?;
        Ok(Self {
            tasks_path: data_dir.join("tasks.tsv"),
            settings_path: data_dir.join("settings.ini"),
            summaries_dir,
            data_dir,
        })
    }

    pub fn load_tasks(&self) -> Vec<Task> {
        let Ok(text) = fs::read_to_string(&self.tasks_path) else {
            return Vec::new();
        };

        text.lines()
            .filter_map(|line| {
                let trimmed = line.trim_end();
                if trimmed.is_empty() || trimmed.starts_with("id\t") {
                    return None;
                }
                parse_task_line(trimmed)
            })
            .collect()
    }

    pub fn save_tasks(&self, tasks: &[Task]) -> io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        let mut out = String::from("id\tcreated_at\tdue\tcompleted\tcompleted_at\ttitle\tnote\n");
        for task in tasks {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                task.id,
                task.created_at.storage_string(),
                task.due.storage_string(),
                if task.completed { "1" } else { "0" },
                task.completed_at
                    .map(|dt| dt.storage_string())
                    .unwrap_or_default(),
                escape_field(&task.title),
                escape_field(&task.note)
            ));
        }
        fs::write(&self.tasks_path, out)
    }

    pub fn load_settings(&self) -> Settings {
        let Ok(text) = fs::read_to_string(&self.settings_path) else {
            return Settings::default();
        };
        let mut settings = Settings::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = unescape_field(value.trim_end());
            match key.trim() {
                "summary_weekday" => {
                    settings.summary_weekday = value.parse::<u32>().unwrap_or(5).clamp(1, 7);
                }
                "auto_summary" => settings.auto_summary = value == "1",
                "launch_on_startup" => settings.launch_on_startup = value == "1",
                "background_color" => settings.background_color = value,
                "window_alpha" => {
                    settings.window_alpha = value.parse::<u16>().unwrap_or(232).clamp(30, 255) as u8
                }
                "llm_api_url" => settings.llm_api_url = value,
                "llm_model" => settings.llm_model = value,
                "llm_api_key" => settings.llm_api_key = value,
                "last_summary_date" => settings.last_summary_date = value,
                "window_x" => settings.window_x = value.parse::<i32>().unwrap_or(80),
                "window_y" => settings.window_y = value.parse::<i32>().unwrap_or(80),
                "window_width" => {
                    settings.window_width = value.parse::<i32>().unwrap_or(420).max(320)
                }
                "window_height" => {
                    settings.window_height = value.parse::<i32>().unwrap_or(620).max(420)
                }
                _ => {}
            }
        }
        settings
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        let text = format!(
            "summary_weekday={}\nauto_summary={}\nlaunch_on_startup={}\nbackground_color={}\nwindow_alpha={}\nllm_api_url={}\nllm_model={}\nllm_api_key={}\nlast_summary_date={}\nwindow_x={}\nwindow_y={}\nwindow_width={}\nwindow_height={}\n",
            settings.summary_weekday,
            if settings.auto_summary { "1" } else { "0" },
            if settings.launch_on_startup { "1" } else { "0" },
            escape_field(&settings.background_color),
            settings.window_alpha,
            escape_field(&settings.llm_api_url),
            escape_field(&settings.llm_model),
            escape_field(&settings.llm_api_key),
            escape_field(&settings.last_summary_date),
            settings.window_x,
            settings.window_y,
            settings.window_width,
            settings.window_height
        );
        fs::write(&self.settings_path, text)
    }

    pub fn write_summary(&self, date: &str, text: &str) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.summaries_dir)?;
        let path = self.summaries_dir.join(format!("summary-{}.md", date));
        fs::write(&path, text)?;
        Ok(path)
    }
}

pub fn next_task_id(tasks: &[Task]) -> u64 {
    tasks.iter().map(|task| task.id).max().unwrap_or(0) + 1
}

fn parse_task_line(line: &str) -> Option<Task> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 7 {
        return None;
    }
    let id = fields[0].parse::<u64>().ok()?;
    let created_at = LocalDateTime::parse(fields[1]).ok()?;
    let due = LocalDateTime::parse(fields[2]).ok()?;
    let completed = fields[3] == "1";
    let completed_at = if fields[4].trim().is_empty() {
        None
    } else {
        LocalDateTime::parse(fields[4]).ok()
    };
    Some(Task {
        id,
        created_at,
        due,
        completed,
        completed_at,
        title: unescape_field(fields[5]),
        note: unescape_field(fields[6]),
    })
}

fn escape_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_field(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
