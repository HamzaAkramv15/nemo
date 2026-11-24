// Профиль питания (power-profiles-daemon, он же tuned-ppd): экономия / баланс / производительность.
// Читаем и меняем по системной шине короткими нитями; без демона модуль бара прячется.

use crate::app::{App, Msg, SKind};
use calloop::channel::Sender;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use zbus::blocking::Proxy;
use zbus::zvariant::{OwnedValue, Value};

/// (имя на шине, путь, интерфейс): новое имя и старое
const BUS: &[(&str, &str, &str)] = &[
    ("org.freedesktop.UPower.PowerProfiles", "/org/freedesktop/UPower/PowerProfiles", "org.freedesktop.UPower.PowerProfiles"),
    ("net.hadess.PowerProfiles", "/net/hadess/PowerProfiles", "net.hadess.PowerProfiles"),
];

/// порядок, значок и подпись профилей
pub const KINDS: &[(&str, &str, &str)] = &[("power-saver", "leaf", "экономия"), ("balanced", "gauge", "баланс"), ("performance", "bolt", "производительность")];

static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Default, Clone, PartialEq)]
pub struct Prof {
    pub present: bool,
    pub active: String,
    pub list: Vec<String>,
    /// почему «производительность» урезана (перегрев, ноутбук на коленях); пусто — всё в порядке
    pub degraded: String,
}

impl Prof {
    pub fn icon(&self) -> &'static str {
        KINDS.iter().find(|k| k.0 == self.active).map(|k| k.1).unwrap_or("gauge")
    }
}

fn proxy<'a>(c: &'a zbus::blocking::Connection) -> Option<Proxy<'a>> {
    for (name, path, iface) in BUS {
        if let Ok(p) = Proxy::new(c, *name, *path, *iface) {
            if p.get_property::<String>("ActiveProfile").is_ok() {
                return Some(p);
            }
        }
    }
    None
}

fn query() -> Prof {
    let Some(c) = crate::net::system() else { return Prof::default() };
    let Some(p) = proxy(&c) else { return Prof::default() };
    let list = p
        .get_property::<Vec<HashMap<String, OwnedValue>>>("Profiles")
        .unwrap_or_default()
        .iter()
        .filter_map(|m| match m.get("Profile").map(|v| &**v) {
            Some(Value::Str(s)) => Some(s.to_string()),
            _ => None,
        })
        .collect();
    Prof { present: true, active: p.get_property("ActiveProfile").unwrap_or_default(), list, degraded: p.get_property("PerformanceDegraded").unwrap_or_default() }
}

/// перечитать (и, если задан, сначала включить профиль); один запрос за раз
pub fn refresh(set: Option<String>, tx: Sender<Msg>) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        if let (Some(v), Some(c)) = (set, crate::net::system()) {
            if let Some(p) = proxy(&c) {
                let _ = p.set_property("ActiveProfile", v);
            }
        }
        let _ = tx.send(Msg::Prof(query()));
        BUSY.store(false, Ordering::SeqCst);
    });
}

impl App {
    pub fn on_prof(&mut self, p: Prof) {
        if p != self.st.prof {
            self.st.prof = p;
            self.dirty(&SKind::Bar);
            self.dirty(&SKind::Popup);
        }
    }

    /// следующий (или предыдущий) профиль по кругу
    pub fn prof_step(&mut self, d: i32) {
        let ps: Vec<&str> = KINDS.iter().map(|k| k.0).filter(|k| self.st.prof.list.iter().any(|l| l == k)).collect();
        if ps.is_empty() {
            return;
        }
        let i = ps.iter().position(|k| *k == self.st.prof.active).unwrap_or(0) as i32;
        let n = ps[(i + d).rem_euclid(ps.len() as i32) as usize].to_string();
        self.prof_set(n);
    }

    /// правило «на батарее — экономия, от сети — баланс»: срабатывает при смене питания и один раз на старте;
    /// игра в фокусе его не трогает (у неё свой профиль), недоступный профиль пропускается
    pub fn power_policy(&mut self) {
        let Some(b) = self.st.sys.bat else { return };
        let ac = b.charging || b.plugged;
        if self.st.pol_prev == Some(ac) {
            return;
        }
        self.st.pol_prev = Some(ac);
        let want = if ac { self.st.set.profile_ac.clone() } else { self.st.set.profile_battery.clone() };
        if let Some(p) = policy_target(&want, self.st.game.game, &self.st.prof) {
            self.prof_set(p);
        }
    }

    pub fn prof_set(&mut self, p: String) {
        // сразу показать выбор, демон подтвердит
        self.st.prof.active = p.clone();
        self.dirty(&SKind::Bar);
        self.dirty(&SKind::Popup);
        refresh(Some(p), self.tx.clone());
    }
}

/// какой профиль включить по правилу; None — ничего менять не нужно
pub fn policy_target(want: &str, in_game: bool, now: &Prof) -> Option<String> {
    if want.is_empty() || in_game || !now.present || now.active == want || !now.list.iter().any(|l| l == want) {
        return None;
    }
    Some(want.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prof(active: &str) -> Prof {
        Prof { present: true, active: active.into(), list: vec!["power-saver".into(), "balanced".into(), "performance".into()], degraded: String::new() }
    }

    #[test]
    fn the_rule_changes_only_what_needs_changing() {
        assert_eq!(policy_target("power-saver", false, &prof("balanced")).as_deref(), Some("power-saver"));
        assert_eq!(policy_target("", false, &prof("balanced")), None, "no rule: leave alone");
        assert_eq!(policy_target("balanced", false, &prof("balanced")), None, "already there");
        assert_eq!(policy_target("power-saver", true, &prof("performance")), None, "a game keeps its own profile");
        let mut absent = prof("balanced");
        absent.present = false;
        assert_eq!(policy_target("power-saver", false, &absent), None, "no power-profiles-daemon");
        let mut two = prof("balanced");
        two.list = vec!["balanced".into(), "performance".into()];
        assert_eq!(policy_target("power-saver", false, &two), None, "the machine lacks that profile");
    }
}
