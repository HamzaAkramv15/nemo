// Ночной свет: вечером цвета теплее через wlsunset (wlr-gamma-control, он у niri есть). От заката
// до восхода по координатам или по часам, если координат нет. Процесс принадлежит шеллу и
// умирает вместе с ним. Модуль бара `night` показывает и переключает.
// i18n: inline pairs

use crate::app::{App, SKind};
use std::process::{Child, Command, Stdio};

#[derive(Default)]
pub struct Night {
    /// работал при последней проверке (по этому рисует бар, обновляет tick)
    pub on: bool,
    child: Option<Child>,
    args: Vec<String>,
    /// место по часовому поясу уже спрашивают (locate.rs): не запускать вторую нить
    locating: bool,
}

impl Night {
    pub fn running(&mut self) -> bool {
        match &mut self.child {
            Some(c) => matches!(c.try_wait(), Ok(None)),
            None => false,
        }
    }
    fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            // wlsunset сам возвращает гамму, когда завершается
            unsafe { libc::kill(c.id() as i32, libc::SIGTERM) };
            let _ = c.wait();
        }
    }
}

impl Drop for Night {
    fn drop(&mut self) {
        self.stop();
    }
}

/// «55.75, 37.62», «55.75 37.62», «55.75 ,37.62» → (широта, долгота)
pub fn geo(s: &str) -> Option<(f64, f64)> {
    let mut it = s.split(|c: char| c == ',' || c == ';' || c.is_whitespace()).filter(|x| !x.is_empty());
    let (a, b) = (it.next()?, it.next()?);
    if it.next().is_some() {
        return None;
    }
    let (lat, lon) = (a.parse::<f64>().ok()?, b.parse::<f64>().ok()?);
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

/// аргументы wlsunset по настройкам
pub fn args(set: &crate::config::Settings) -> Vec<String> {
    // auto: координаты из locate.rs (часовой пояс системы); пока их нет — по часам
    let place = if crate::locate::is_auto(&set.night_geo) { crate::locate::cached().map(|(la, lo, _)| (la, lo)) } else { geo(&set.night_geo) };
    yami_px::night::args(set.num("night_temp"), set.night_always, place, set.num("night_on_h"), set.num("night_off_h"))
}

impl App {
    pub fn night_sync(&mut self) {
        let s = &self.st.set;
        if !s.night || !crate::doctor::which("wlsunset") {
            if self.st.night.on {
                self.st.night.stop();
                self.st.night.on = false;
                self.dirty(&SKind::Bar);
            }
            return;
        }
        // «auto» и места ещё нет — узнать в фоне и перечитать настройки, когда узнали
        if crate::locate::is_auto(&s.night_geo) && crate::locate::cached().is_none() && !self.st.night.locating {
            self.st.night.locating = true;
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                if crate::locate::resolve().is_ok() {
                    let _ = tx.send(crate::app::Msg::Ipc("reload".into()));
                }
            });
        }
        let s = &self.st.set;
        let a = args(s);
        if self.st.night.running() && self.st.night.args == a {
            return;
        }
        self.st.night.stop();
        let mut cmd = Command::new("wlsunset");
        cmd.args(&a).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        self.st.night.child = crate::proc::owned(&mut cmd).spawn().ok();
        self.st.night.on = self.st.night.child.is_some();
        self.st.night.args = a;
        self.dirty(&SKind::Bar);
    }

    /// `yami night on|off|toggle|always|schedule|temp K`: то же, что в настройках; сохраняется в shell.json
    pub fn night_cmd(&mut self, arg: &str) {
        if !crate::doctor::which("wlsunset") {
            self.toast_note("ночной свет", "нужен wlsunset: поставь его из своего дистрибутива");
            return;
        }
        let s = &mut self.st.set;
        match arg.split_once(' ').unwrap_or((arg, "")) {
            ("on", _) => s.night = true,
            ("off", _) => s.night = false,
            ("toggle", _) => s.night = !s.night,
            // «always» включает сразу: на команду, которая молча ничего не меняет, не похоже
            ("always", _) => {
                s.night = true;
                s.night_always = true;
            }
            ("schedule", _) => s.night_always = false,
            ("temp", k) => match k.trim().parse::<i32>() {
                Ok(k) => s.night_temp = k.clamp(2500, 6000) / 250 * 250,
                Err(_) => return,
            },
            _ => return,
        }
        self.settings_changed();
        self.save_now();
    }

    /// правая кнопка на луне: следующая температура из набора (тёплый вечер → очень тёплый → мягкий), и включить
    pub fn night_preset(&mut self) {
        if !crate::doctor::which("wlsunset") {
            self.toast_note("ночной свет", "нужен wlsunset: поставь его из своего дистрибутива");
            return;
        }
        self.st.set.night = true;
        self.st.set.night_temp = next_temp(self.st.set.night_temp);
        self.settings_changed();
        self.toast_note("ночной свет", &crate::tf!("{} К", "{} K", "{} K", self.st.set.night_temp));
    }

    pub fn night_toggle(&mut self) {
        if !crate::doctor::which("wlsunset") {
            self.toast_note("ночной свет", "нужен wlsunset: поставь его из своего дистрибутива");
            return;
        }
        self.st.set.night = !self.st.set.night;
        self.settings_changed();
    }
}

/// температуры правой кнопки, кельвины: от мягкой к тёплой, и по кругу
pub const TEMPS: [i32; 4] = [4500, 3500, 2800, 2500];

/// следующая температура из набора: ближайшая ниже текущей, а ниже всех — снова самая мягкая
pub fn next_temp(cur: i32) -> i32 {
    TEMPS.iter().copied().find(|&t| t < cur).unwrap_or(TEMPS[0])
}

#[cfg(test)]
mod tests {
    #[test]
    fn temp_presets_cycle_downwards() {
        assert_eq!(super::next_temp(6000), 4500);
        assert_eq!(super::next_temp(4500), 3500);
        assert_eq!(super::next_temp(4000), 3500);
        assert_eq!(super::next_temp(2800), 2500);
        assert_eq!(super::next_temp(2500), 4500);
        assert!(super::TEMPS.iter().all(|t| (2500..=6000).contains(t)));
    }

    #[test]
    fn coords() {
        assert_eq!(super::geo("55.75, 37.62"), Some((55.75, 37.62)));
        assert_eq!(super::geo("55.75 37.62"), Some((55.75, 37.62)));
        assert_eq!(super::geo("55.75 ,37.62"), Some((55.75, 37.62)));
        assert_eq!(super::geo("-33.9,18.4"), Some((-33.9, 18.4)));
        assert_eq!(super::geo("1, 2, 3"), None);
        assert_eq!(super::geo("95, 0"), None);
        assert_eq!(super::geo(""), None);
    }

    #[test]
    fn wlsunset_args() {
        let mut s = crate::config::Settings { night_temp: 6500, ..Default::default() };
        let a = super::args(&s);
        assert_eq!(&a[..4], ["-t", "6000", "-T", "6500"]);
        s.night_on_h = 7;
        s.night_off_h = 7;
        let a = super::args(&s);
        assert_eq!(&a[4..], ["-S", "07:00", "-s", "08:00"]);
        // всегда включён: расписания нет, высокая температура всего на 1 K выше низкой
        s.night_always = true;
        s.night_temp = 3500;
        assert_eq!(super::args(&s), ["-t", "3500", "-T", "3501"]);
    }
}
