// Подсветка клавиатуры гаснет, когда не печатаешь, и возвращается с первой клавишей. Замер (ASUS TUF, батарея, паспорт 10-11 Вт): подсветка на
// втором уровне из трёх тянула ≈ 1,5 Вт — больше, чем любое другое, что шелл может выключить сам. Работает через простой композитора
// (ext-idle-notify, как блокировка и гашение экрана) и logind (`Session.SetBrightness`): root не нужен. Режимы настройки `kbdLight`:
//   keep — не трогать; idle — гаснет при простое всегда; idle_battery (по умолчанию) — только пока работаем от батареи.

use crate::app::App;
use std::fs;

/// светодиод подсветки клавиатуры: (имя в /sys/class/leds, наибольшая яркость)
pub fn find() -> Option<(String, u32)> {
    let rd = fs::read_dir("/sys/class/leds").ok()?;
    rd.flatten().find_map(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.contains("kbd_backlight") {
            return None;
        }
        let max = fs::read_to_string(e.path().join("max_brightness")).ok()?.trim().parse::<u32>().ok().filter(|m| *m > 0)?;
        Some((name, max))
    })
}

/// есть ли у машины подсветка клавиатуры, которой можно управлять (один раз за запуск)
pub fn present() -> bool {
    static P: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    // NEMO_NO_KBD: вложенный niri для замеров и снимки состояния делят сеанс с настоящим: их простой не должен гасить настоящую подсветку
    *P.get_or_init(|| std::env::var_os("NEMO_NO_KBD").is_none() && find().is_some())
}

fn level(name: &str) -> u32 {
    fs::read_to_string(format!("/sys/class/leds/{name}/brightness")).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

/// выставить яркость через logind; в нити, чтобы зависший вызов не держал цикл событий
fn set(name: &str, value: u32) {
    let name = name.to_string();
    std::thread::spawn(move || {
        let Ok(c) = zbus::blocking::Connection::system() else { return };
        let _ = c.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1/session/auto",
            Some("org.freedesktop.login1.Session"),
            "SetBrightness",
            &("leds", name.as_str(), value),
        );
    });
}

/// сколько секунд простоя до того, как подсветка гаснет; 0 — не гасить (режим keep или подсветки нет)
pub fn idle_secs(s: &crate::config::Settings) -> u32 {
    if s.kbd_light == "keep" || !present() {
        0
    } else {
        s.kbd_idle_secs.clamp(5, 600)
    }
}

/// гасить ли сейчас: режим и питание
pub fn should_dim(policy: &str, on_battery: bool) -> bool {
    match policy {
        "idle" => true,
        "idle_battery" => on_battery,
        _ => false,
    }
}

impl App {
    /// простой наступил: запомнить уровень и погасить
    pub fn kbd_idle(&mut self) {
        let on_battery = self.st.sys.bat.is_some_and(|b| !b.charging && !b.plugged);
        if !should_dim(&self.st.set.kbd_light, on_battery) {
            return;
        }
        let Some((name, _)) = find() else { return };
        let cur = level(&name);
        if cur > 0 {
            self.st.kbd_saved = Some(cur);
            set(&name, 0);
        }
    }

    /// первое нажатие после простоя: вернуть уровень (если за это время его не меняли руками)
    pub fn kbd_resume(&mut self) {
        let Some(v) = self.st.kbd_saved.take() else { return };
        let Some((name, _)) = find() else { return };
        if level(&name) == 0 {
            set(&name, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modes_decide_when_the_light_goes_out() {
        assert!(!should_dim("keep", true) && !should_dim("keep", false));
        assert!(should_dim("idle", true) && should_dim("idle", false));
        assert!(should_dim("idle_battery", true) && !should_dim("idle_battery", false), "on the charger the light stays");
        assert!(!should_dim("nonsense", true));
    }

    #[test]
    fn keep_means_no_idle_timer() {
        let mut s = crate::config::Settings::default();
        s.kbd_light = "keep".into();
        assert_eq!(idle_secs(&s), 0);
    }
}
