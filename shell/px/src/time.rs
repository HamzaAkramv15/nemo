// Местное время без chrono (libc localtime_r) и русские названия дней и месяцев: шелл, экран входа.

#[derive(Clone, Copy, Debug)]
pub struct Tm {
    pub year: i32,
    pub mon: u32,
    pub day: u32,
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
    pub wday: u32,
}

/// секунды с 1970-го (метки кеша, возраст данных)
pub fn unix_now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn local_now() -> Tm {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    local_tm(now)
}
pub fn local_tm(t: i64) -> Tm {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    Tm { year: tm.tm_year + 1900, mon: tm.tm_mon as u32 + 1, day: tm.tm_mday as u32, hour: tm.tm_hour as u32, min: tm.tm_min as u32, sec: tm.tm_sec as u32, wday: tm.tm_wday as u32 }
}

pub const RU_DAY: [&str; 7] = ["вс", "пн", "вт", "ср", "чт", "пт", "сб"];
pub const RU_DAY_FULL: [&str; 7] = ["воскресенье", "понедельник", "вторник", "среда", "четверг", "пятница", "суббота"];
pub const RU_MON: [&str; 12] = ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];
pub const RU_MON_NOM: [&str; 12] = ["январь", "февраль", "март", "апрель", "май", "июнь", "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"];

static HOUR12: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 12-часовой формат (AM/PM): выбирает шелл по настройке `clock12`, дальше он общий для всех часов
pub fn set_hour12(on: bool) {
    HOUR12.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn hour12() -> bool {
    HOUR12.load(std::sync::atomic::Ordering::Relaxed)
}

/// час на циферблате и AM/PM к нему: 0 → 12 AM, 12 → 12 PM, 13 → 1 PM
pub fn hour_12(h: u32) -> (u32, &'static str) {
    (if h % 12 == 0 { 12 } else { h % 12 }, if h < 12 { "AM" } else { "PM" })
}

/// час как его пишут на часах: «07» в 24-часовом, «7» в 12-часовом
pub fn hour_text(tm: &Tm) -> String {
    if hour12() {
        hour_12(tm.hour).0.to_string()
    } else {
        format!("{:02}", tm.hour)
    }
}

/// AM/PM, когда формат 12-часовой
pub fn ampm(tm: &Tm) -> Option<&'static str> {
    hour12().then(|| hour_12(tm.hour).1)
}

/// «14:05», «14:05:09», «2:05 PM» или «2:05:09 PM»
pub fn clock_text(tm: &Tm, seconds: bool) -> String {
    let mut s = format!("{}:{:02}", hour_text(tm), tm.min);
    if seconds {
        s.push_str(&format!(":{:02}", tm.sec));
    }
    if let Some(p) = ampm(tm) {
        s.push(' ');
        s.push_str(p);
    }
    s
}

/// короткая дата для бара: `dm` «пн 17.11», `md` «Mon 11/17», `iso` «2026-11-17»; `wday` уже переведён
pub fn date_short(tm: &Tm, style: &str, wday: &str) -> String {
    match style {
        "md" => format!("{wday} {:02}/{:02}", tm.mon, tm.day),
        "iso" => format!("{}-{:02}-{:02}", tm.year, tm.mon, tm.day),
        _ => format!("{wday} {:02}.{:02}", tm.day, tm.mon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tm(hour: u32, min: u32, sec: u32) -> Tm {
        Tm { year: 2026, mon: 11, day: 7, hour, min, sec, wday: 6 }
    }

    #[test]
    fn twelve_hour_clock() {
        assert_eq!(hour_12(0), (12, "AM"));
        assert_eq!(hour_12(1), (1, "AM"));
        assert_eq!(hour_12(11), (11, "AM"));
        assert_eq!(hour_12(12), (12, "PM"));
        assert_eq!(hour_12(13), (1, "PM"));
        assert_eq!(hour_12(23), (11, "PM"));
    }

    #[test]
    fn clock_formats() {
        // HOUR12 общий на процесс: оба формата проверяются в одном тесте, не в двух параллельных
        set_hour12(false);
        assert_eq!(clock_text(&tm(7, 5, 9), false), "07:05");
        assert_eq!(clock_text(&tm(15, 5, 9), true), "15:05:09");
        assert_eq!(ampm(&tm(15, 5, 9)), None);
        set_hour12(true);
        assert_eq!(clock_text(&tm(0, 5, 9), false), "12:05 AM");
        assert_eq!(clock_text(&tm(15, 5, 9), false), "3:05 PM");
        assert_eq!(clock_text(&tm(15, 5, 9), true), "3:05:09 PM");
        assert_eq!(hour_text(&tm(7, 0, 0)), "7");
        set_hour12(false);
    }

    #[test]
    fn short_dates() {
        let t = tm(0, 0, 0);
        assert_eq!(date_short(&t, "dm", "Sat"), "Sat 07.11");
        assert_eq!(date_short(&t, "md", "Sat"), "Sat 11/07");
        assert_eq!(date_short(&t, "iso", "Sat"), "2026-11-07");
        assert_eq!(date_short(&t, "", "Sat"), "Sat 07.11");
    }
}
