// Кофе: пока включён, экран не гаснет и не запирается по бездействию, а система не засыпает сама.
// Две половины: шелл не выполняет свои «простой → запереть / погасить экран» (wayland.rs, Idled), а у
// logind берётся блокировка «idle:sleep» (Inhibit … "block"): она не даёт ему самому усыпить машину
// по таймеру и держится, пока жива нить. Выключили (или вышло время) — нить отпускает блокировку,
// и шелл заводит таймеры простоя заново. Крышка ноутбука по-прежнему усыпляет и запирает: кофе —
// про простой, не про закрытый ноутбук в сумке. Включается из бара, плиткой панели управления,
// `yami caffeine on|off|toggle|N` (N минут). Не переживает перезапуск шелла: это «сейчас», не настройка.
// i18n: inline pairs

use crate::app::{App, SKind};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use zbus::blocking::Proxy;

#[derive(Default)]
pub struct Caffeine {
    pub on: bool,
    /// когда выключится сам (None — когда выключат)
    pub until: Option<Instant>,
    /// пока этот отправитель жив — нить держит блокировку logind; убрали — она отпускает
    hold: Option<mpsc::Sender<()>>,
}

impl Caffeine {
    /// сколько осталось, мин (округлением вверх); None — без срока
    pub fn left_min(&self) -> Option<u64> {
        self.until.map(|t| t.saturating_duration_since(Instant::now()).as_secs().div_ceil(60))
    }
}

/// нить, что держит у logind блокировку простоя и сна, пока не придёт сигнал или не уйдёт отправитель
fn hold() -> mpsc::Sender<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = std::thread::Builder::new().name("caffeine".into()).spawn(move || {
        let fd = crate::net::system().and_then(|c| {
            let p = Proxy::new(&c, "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager").ok()?;
            let m = p.call_method("Inhibit", &("idle:sleep", "Nemo", crate::i18n::tr("кофе: не гасить экран и не засыпать"), "block")).ok()?;
            m.body().deserialize::<zbus::zvariant::OwnedFd>().ok()
        });
        // Err — отправитель ушёл (выключили или шелл завершился): отпускаем
        let _ = rx.recv();
        drop(fd);
    });
    tx
}

/// «30», «30m», «2h», «1h30m»-не-надо: минуты числом, с «m» или часы с «h»; 0 и мусор — None
/// сроки правой кнопки, минуты; 0 — без срока
pub const PRESETS: [u32; 5] = [30, 60, 120, 240, 0];

/// что включить следующим: выключено — первый срок; идёт срок — ближайший больше остатка; без срока — выключить (None)
pub fn next_preset(on: bool, left: Option<u64>) -> Option<u32> {
    if !on {
        return Some(PRESETS[0]);
    }
    let Some(left) = left else { return None };
    PRESETS.iter().copied().find(|&p| p != 0 && p as u64 > left).or(Some(0))
}

/// «30 мин» / «2 ч»
fn fmt_span(m: u32) -> String {
    if m >= 60 && m % 60 == 0 { format!("{}h", m / 60) } else { format!("{m}m") }
}

pub fn parse_minutes(s: &str) -> Option<u32> {
    let s = s.trim();
    let (num, mult) = match s.strip_suffix('h') {
        Some(n) => (n, 60),
        None => (s.strip_suffix('m').unwrap_or(s), 1),
    };
    num.parse::<u32>().ok().filter(|n| *n > 0).map(|n| (n * mult).min(24 * 60))
}

impl App {
    /// включить (minutes = 0 — без срока) или выключить
    pub fn caffeine_set(&mut self, on: bool, minutes: u32) {
        let c = &mut self.st.caffeine;
        if on {
            if c.hold.is_none() {
                c.hold = Some(hold());
            }
            c.on = true;
            c.until = (minutes > 0).then(|| Instant::now() + Duration::from_secs(minutes as u64 * 60));
        } else if c.on {
            c.on = false;
            c.until = None;
            c.hold = None;
            // простой считался заново с последнего движения: заводим таймеры сначала
            self.setup_idle();
        }
        self.dirty(&SKind::Bar);
        self.dirty(&SKind::Popup);
    }

    /// клик по значку: выключить, а включить — на срок из настроек (0 — без срока)
    pub fn caffeine_toggle(&mut self) {
        let (on, minutes) = (!self.st.caffeine.on, self.st.set.num("caffeine_minutes").max(0) as u32);
        self.caffeine_set(on, minutes);
    }

    /// правая кнопка на чашке: следующий срок из набора (потом «пока не выключу», потом выкл.)
    pub fn caffeine_preset(&mut self) {
        let left = self.st.caffeine.left_min();
        let on = self.st.caffeine.on;
        match next_preset(on, left) {
            Some(m) => {
                self.caffeine_set(true, m);
                let say = if m == 0 { "кофе включён, пока не выключишь".to_string() } else { crate::tf!("кофе включён на {}", "caffeine on for {}", "café ligado por {}", fmt_span(m)) };
                self.toast_note("кофе", &say);
            }
            None => self.caffeine_set(false, 0),
        }
    }

    /// `yami caffeine on|off|toggle|N`
    pub fn caffeine_cmd(&mut self, arg: &str) {
        match arg.trim() {
            "on" => self.caffeine_set(true, 0),
            "off" => self.caffeine_set(false, 0),
            "toggle" => self.caffeine_toggle(),
            n => {
                if let Some(m) = parse_minutes(n) {
                    self.caffeine_set(true, m);
                }
            }
        }
    }

    /// раз в тик: вышел срок — выключить
    pub fn caffeine_tick(&mut self) {
        if self.st.caffeine.until.is_some_and(|t| Instant::now() >= t) {
            self.caffeine_set(false, 0);
            let q = self.quip_for(crate::personality::Event::CaffeineEnded);
            self.toast_note_plus("кофе", "кофе выключен: срок вышел", q);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn right_click_presets_walk_up_then_unlimited_then_off() {
        use super::next_preset as n;
        assert_eq!(n(false, None), Some(30));
        assert_eq!(n(true, Some(25)), Some(30));
        assert_eq!(n(true, Some(30)), Some(60));
        assert_eq!(n(true, Some(59)), Some(60));
        assert_eq!(n(true, Some(200)), Some(240));
        assert_eq!(n(true, Some(240)), Some(0));
        assert_eq!(n(true, None), None);
    }

    #[test]
    fn minutes_are_parsed() {
        assert_eq!(super::parse_minutes("30"), Some(30));
        assert_eq!(super::parse_minutes("45m"), Some(45));
        assert_eq!(super::parse_minutes("2h"), Some(120));
        assert_eq!(super::parse_minutes(" 15 "), Some(15));
        // потолок — сутки, ноль и мусор — не срок
        assert_eq!(super::parse_minutes("99h"), Some(1440));
        assert_eq!(super::parse_minutes("0"), None);
        assert_eq!(super::parse_minutes("soon"), None);
        assert_eq!(super::parse_minutes("-5"), None);
    }
}
