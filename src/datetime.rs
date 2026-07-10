use crate::win32::{GetLocalTime, SYSTEMTIME};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl LocalDateTime {
    pub fn now() -> Self {
        let mut st = SYSTEMTIME::default();
        unsafe {
            GetLocalTime(&mut st);
        }
        Self {
            year: st.wYear as i32,
            month: st.wMonth as u32,
            day: st.wDay as u32,
            hour: st.wHour as u32,
            minute: st.wMinute as u32,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::parse_with_default(value, 18, 0)
    }

    pub fn parse_start_bound(value: &str) -> Result<Option<Self>, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        Self::parse_with_default(trimmed, 0, 0).map(Some)
    }

    fn parse_with_default(
        value: &str,
        default_hour: u32,
        default_minute: u32,
    ) -> Result<Self, String> {
        let normalized = value.trim().replace('/', "-");
        let mut parts = normalized.split_whitespace();
        let date_part = parts
            .next()
            .ok_or_else(|| "请输入日期，例如 2026-07-07 18:30".to_string())?;
        let time_part = parts.next();
        if parts.next().is_some() {
            return Err("时间格式过长，请使用 2026-07-07 18:30".to_string());
        }

        let date_bits: Vec<&str> = date_part.split('-').collect();
        if date_bits.len() != 3 {
            return Err("日期格式应为 YYYY-MM-DD".to_string());
        }

        let year = parse_i32(date_bits[0], "年份")?;
        let month = parse_u32(date_bits[1], "月份")?;
        let day = parse_u32(date_bits[2], "日期")?;
        let (hour, minute) = if let Some(time) = time_part {
            let time_bits: Vec<&str> = time.split(':').collect();
            if time_bits.len() != 2 {
                return Err("时间格式应为 HH:MM".to_string());
            }
            (
                parse_u32(time_bits[0], "小时")?,
                parse_u32(time_bits[1], "分钟")?,
            )
        } else {
            (default_hour, default_minute)
        };

        let candidate = Self {
            year,
            month,
            day,
            hour,
            minute,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=12).contains(&self.month) {
            return Err("月份必须在 1 到 12 之间".to_string());
        }
        let max_day = days_in_month(self.year, self.month);
        if self.day == 0 || self.day > max_day {
            return Err(format!("{} 月最多只有 {} 天", self.month, max_day));
        }
        if self.hour > 23 {
            return Err("小时必须在 0 到 23 之间".to_string());
        }
        if self.minute > 59 {
            return Err("分钟必须在 0 到 59 之间".to_string());
        }
        Ok(())
    }

    pub fn minutes_key(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * 1440
            + self.hour as i64 * 60
            + self.minute as i64
    }

    pub fn diff_minutes_from(&self, other: Self) -> i64 {
        self.minutes_key() - other.minutes_key()
    }

    pub fn add_days(&self, days: i64) -> Self {
        let target_days = days_from_civil(self.year, self.month, self.day) + days;
        let (year, month, day) = civil_from_days(target_days);
        Self {
            year,
            month,
            day,
            hour: self.hour,
            minute: self.minute,
        }
    }

    pub fn weekday_mon1(&self) -> u32 {
        let days = days_from_civil(self.year, self.month, self.day);
        (days + 3).rem_euclid(7) as u32 + 1
    }

    pub fn date_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn storage_string(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }

    pub fn short_string(&self) -> String {
        format!(
            "{:02}-{:02} {:02}:{:02}",
            self.month, self.day, self.hour, self.minute
        )
    }

    pub fn cn_weekday(&self) -> &'static str {
        weekday_name(self.weekday_mon1())
    }
}

impl Ord for LocalDateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.minutes_key().cmp(&other.minutes_key())
    }
}

impl PartialOrd for LocalDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn weekday_name(weekday: u32) -> &'static str {
    match weekday {
        1 => "星期一",
        2 => "星期二",
        3 => "星期三",
        4 => "星期四",
        5 => "星期五",
        6 => "星期六",
        7 => "星期日",
        _ => "未知",
    }
}

fn parse_i32(value: &str, label: &str) -> Result<i32, String> {
    value
        .parse::<i32>()
        .map_err(|_| format!("{}不是有效数字", label))
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("{}不是有效数字", label))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}
