// Телефон через KDE Connect (настройки → телефон): список устройств и «позвонить на телефон». Сам Nemo ничего не строит
// поверх: демон kdeconnectd и приложение на телефоне — KDE Connect, мы только читаем `kdeconnect-cli -l` и зовём его команды.
// Проверка идёт в нити и только когда страница открыта; нет kdeconnect-cli — страница говорит, что поставить.

use crate::app::{Msg, State};
use crate::font::PX;
use crate::gfx::R;
use crate::settings::title;
use crate::ui::{Act, Tone, Ui};
use calloop::channel::Sender;
use std::process::{Command, Stdio};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dev {
    pub id: String,
    pub name: String,
    pub paired: bool,
    pub reachable: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    /// kdeconnect-cli есть в системе
    pub tool: bool,
    pub devs: Vec<Dev>,
}

#[derive(Default)]
pub struct Phone {
    pub found: Option<Found>,
    pub checking: bool,
}

/// строки `kdeconnect-cli -l`: «- Pixel 8: 0123abcd (paired and reachable)»; «0 devices found» и прочее не считаются
pub fn parse_list(text: &str) -> Vec<Dev> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim().strip_prefix("- ")?;
            let (head, state) = l.rsplit_once(" (")?;
            let state = state.trim_end_matches(')');
            let (name, id) = head.rsplit_once(": ")?;
            Some(Dev { id: id.trim().to_string(), name: name.trim().to_string(), paired: state.contains("paired") && !state.contains("unpaired"), reachable: state.contains("reachable") && !state.contains("unreachable") })
        })
        .collect()
}

/// проверка; блокирует нить
pub fn check() -> Found {
    let Some(exe) = nemo_px::paths::find_exe("kdeconnect-cli") else { return Found::default() };
    let out = Command::new(exe).arg("-l").stdin(Stdio::null()).output();
    match out {
        Ok(o) => Found { tool: true, devs: parse_list(&String::from_utf8_lossy(&o.stdout)) },
        Err(_) => Found { tool: true, devs: vec![] },
    }
}

pub fn start(st: &mut State, tx: Sender<Msg>) {
    if st.phone.checking {
        return;
    }
    st.phone.checking = true;
    std::thread::spawn(move || {
        let _ = tx.send(Msg::Phone(check()));
    });
}

/// команда телефону: звонок «найти телефон» или запрос на сопряжение; id — только из нашего же списка (hex), в argv, не в оболочку
pub fn command(id: &str, what: &str) {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return;
    }
    let flag = match what {
        "ring" => "--ring",
        "pair" => "--pair",
        _ => return,
    };
    let _ = Command::new("kdeconnect-cli").args(["-d", id, flag]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

pub fn tab(ui: &mut Ui, st: &State, b: R) {
    let t = ui.t;
    let s = &st.phone;
    let mut y = title(ui, b.x, b.y, b.w, "телефон");
    y += ui.wrap(PX, R::new(b.x, y, b.w, 40), "Телефон через KDE Connect: уведомления, буфер обмена, файлы. Здесь — устройства и «найти телефон».", t.text_dim, 2) + 6;
    let Some(f) = &s.found else {
        ui.text_in(PX, R::new(b.x, y, b.w, 20), if s.checking { "проверяю…" } else { "ещё не проверялось" }, t.text_dim);
        return;
    };
    if !f.tool {
        y += ui.wrap(PX, R::new(b.x, y, b.w, 40), "KDE Connect не установлен: поставь пакет kdeconnect (на телефон — приложение KDE Connect), потом открой эту страницу снова.", t.warn, 2);
        let _ = y;
        return;
    }
    ui.button(R::new(b.x, y, 200, 28), "обновить список", "reboot", Tone::Plain, false, !s.checking, Act::SCycle("phone_refresh"));
    y += 40;
    if f.devs.is_empty() {
        ui.wrap(PX, R::new(b.x, y, b.w, 40), "устройств нет: запусти KDE Connect на телефоне в той же сети и сопряги его (kdeconnect-app)", t.text_faint, 2);
        return;
    }
    for d in &f.devs {
        let col = if d.reachable { t.ok } else { t.text_faint };
        ui.cv.fill(R::new(b.x, y + 8, 6, 6), col);
        ui.text_in(PX, R::new(b.x + 14, y, b.w - 340, 22), &d.name, t.text);
        let state = match (d.paired, d.reachable) {
            (true, true) => "на связи",
            (true, false) => "не в сети",
            (false, true) => "не сопряжён",
            (false, false) => "не в сети",
        };
        ui.text_in(PX, R::new(b.x + b.w - 330, y, 120, 22), state, col);
        if d.paired && d.reachable {
            ui.button(R::new(b.right() - 200, y - 3, 200, 28), "позвонить на телефон", "bell", Tone::Plain, false, true, Act::PhoneRing(d.id.clone()));
        } else if !d.paired && d.reachable {
            ui.button(R::new(b.right() - 200, y - 3, 200, 28), "сопрячь", "bt", Tone::Primary, false, true, Act::PhonePair(d.id.clone()));
        }
        y += 34;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_device_list_is_read_from_kdeconnect_cli() {
        let t = "- Pixel 8: 0123abcd_ef (paired and reachable)\n- Old tablet: 9999 (paired)\n- Stranger: aa11 (reachable)\n0 devices found\nnoise\n";
        let d = parse_list(t);
        assert_eq!(d.len(), 3);
        assert_eq!((d[0].name.as_str(), d[0].id.as_str(), d[0].paired, d[0].reachable), ("Pixel 8", "0123abcd_ef", true, true));
        assert_eq!((d[1].paired, d[1].reachable), (true, false));
        assert_eq!((d[2].paired, d[2].reachable), (false, true));
        assert!(parse_list("0 devices found").is_empty());
    }

    #[test]
    fn only_plain_ids_reach_the_command_line() {
        // не запускает ничего: проверка в том, что вызов с мусором не паникует и не доходит до spawn
        command("a; rm -rf ~", "ring");
        command("", "ring");
        command("abc", "unknown");
    }
}
