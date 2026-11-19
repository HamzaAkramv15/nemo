// Автозапуск (настройки → система → автозапуск): программы, которые сеанс запускает при входе по
// XDG autostart (~/.config/autostart и /etc/xdg/autostart). Список — что запустится на самом деле:
// файл пользователя перекрывает системный с тем же именем, Hidden=true и
// X-GNOME-Autostart-enabled=false выключают запись, OnlyShowIn/NotShowIn решают, для этого ли она
// рабочего стола. Выключение системной записи — маленький файл-пометка у пользователя (с
// комментарием «# nemo:»), включение его убирает; своих файлов человека мы не стираем, а правим две
// строки. Ничего не запускается и не останавливается: меняется только то, что будет при следующем входе.
// i18n: inline pairs

use std::path::{Path, PathBuf};

const MARK: &str = "# nemo:";

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// имя файла: ключ записи («blueman.desktop»)
    pub file: String,
    pub name: String,
    pub exec: String,
    /// запустится при входе
    pub enabled: bool,
    /// запись не для этого рабочего стола (OnlyShowIn/NotShowIn)
    pub other_desktop: bool,
    /// действует файл пользователя (иначе системный)
    pub user: bool,
}

/// значения ключей группы [Desktop Entry]
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut inside = false;
    for l in text.lines().map(str::trim) {
        if l.starts_with('[') {
            inside = l == "[Desktop Entry]";
            continue;
        }
        if inside {
            if let Some(v) = l.strip_prefix(key).and_then(|r| r.strip_prefix('=')) {
                return Some(v.trim());
            }
        }
    }
    None
}

fn listed(v: Option<&str>, desktop: &str) -> Option<bool> {
    let v = v.filter(|s| !s.is_empty())?;
    Some(v.split(';').any(|d| d.eq_ignore_ascii_case(desktop)))
}

/// запись из текста файла; desktop — как называется этот рабочий стол (XDG_CURRENT_DESKTOP)
pub fn parse(file: &str, text: &str, desktop: &str, user: bool) -> Option<Entry> {
    if field(text, "Type").is_some_and(|t| t != "Application") {
        return None;
    }
    let off = field(text, "Hidden") == Some("true") || field(text, "X-GNOME-Autostart-enabled") == Some("false");
    let other_desktop = listed(field(text, "OnlyShowIn"), desktop) == Some(false) || listed(field(text, "NotShowIn"), desktop) == Some(true);
    Some(Entry {
        file: file.to_string(),
        name: field(text, "Name").unwrap_or(file.trim_end_matches(".desktop")).to_string(),
        exec: field(text, "Exec").unwrap_or("").to_string(),
        enabled: !off,
        other_desktop,
        user,
    })
}

/// что сеанс запустит: файлы пользователя перекрывают системные, по алфавиту; не для этого стола — в конце
pub fn scan_dirs(user_dir: &Path, system: &[PathBuf], desktop: &str) -> Vec<Entry> {
    let mut by_file: std::collections::BTreeMap<String, Entry> = Default::default();
    let mut read = |dir: &Path, user: bool| {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if !f.ends_with(".desktop") || by_file.contains_key(&f) {
                continue;
            }
            // файл пользователя читается первым и перекрывает системный; больше 64 КБ и не обычный файл — не запись
            let Ok(m) = e.metadata() else { continue };
            if !m.is_file() || m.len() > 65536 {
                continue;
            }
            if let Some(en) = std::fs::read_to_string(e.path()).ok().and_then(|t| parse(&f, &t, desktop, user)) {
                by_file.insert(f, en);
            }
        }
    };
    read(user_dir, true);
    for d in system {
        read(d, false);
    }
    let mut v: Vec<Entry> = by_file.into_values().collect();
    v.sort_by_key(|e| (e.other_desktop, e.name.to_lowercase()));
    v
}

/// включить или выключить запись; true — изменено. Системную выключает файл-пометка, включение его убирает
pub fn set_enabled(user_dir: &Path, system: &[PathBuf], desktop: &str, file: &str, on: bool) -> bool {
    if file.contains('/') || !file.ends_with(".desktop") || file.starts_with('.') {
        return false;
    }
    let path = user_dir.join(file);
    let in_system = system.iter().any(|d| d.join(file).is_file());
    let Some(cur) = scan_dirs(user_dir, system, desktop).into_iter().find(|e| e.file == file) else { return false };
    if cur.enabled == on {
        return true;
    }
    if std::fs::create_dir_all(user_dir).is_err() {
        return false;
    }
    let text = std::fs::read_to_string(&path).ok();
    match (on, text) {
        // включить: наш файл-пометка уходит; чужой (свой файл человека) теряет только две строки
        (true, Some(t)) if t.contains(MARK) && in_system => std::fs::remove_file(&path).is_ok(),
        (true, Some(t)) => crate::config::write_atomic(&path, &without_switches(&t)).is_ok(),
        (true, None) => true,
        // выключить свой файл: две строки долой и одна своя
        (false, Some(t)) => crate::config::write_atomic(&path, &with_hidden(&without_switches(&t))).is_ok(),
        // выключить системную: пометка
        (false, None) => {
            let body = format!("{MARK} turned off in settings → startup; delete this file to turn it back on\n[Desktop Entry]\nType=Application\nName={}\nHidden=true\n", cur.name.replace('\n', " "));
            crate::config::write_atomic(&path, &body).is_ok()
        }
    }
}

/// текст без строк Hidden и X-GNOME-Autostart-enabled (остальное — как было, комментарии тоже)
fn without_switches(t: &str) -> String {
    let mut out: Vec<&str> = t.lines().filter(|l| !(l.trim().starts_with("Hidden=") || l.trim().starts_with("X-GNOME-Autostart-enabled="))).collect();
    out.push("");
    out.join("\n")
}

/// Hidden=true внутри [Desktop Entry]
fn with_hidden(t: &str) -> String {
    let mut out = vec![];
    let mut done = false;
    for l in t.lines() {
        out.push(l.to_string());
        if l.trim() == "[Desktop Entry]" && !done {
            out.push("Hidden=true".into());
            done = true;
        }
    }
    if !done {
        out.push("[Desktop Entry]".into());
        out.push("Hidden=true".into());
    }
    out.join("\n") + "\n"
}

pub fn user_dir() -> PathBuf {
    crate::config::config_dir().join("autostart")
}

pub fn system_dirs() -> Vec<PathBuf> {
    let dirs = std::env::var("XDG_CONFIG_DIRS").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/etc/xdg".into());
    dirs.split(':').map(|d| PathBuf::from(d).join("autostart")).collect()
}

pub fn desktop() -> String {
    std::env::var("XDG_CURRENT_DESKTOP").ok().and_then(|d| d.split(':').next().map(str::to_string)).filter(|d| !d.is_empty()).unwrap_or_else(|| "niri".into())
}

thread_local! {
    static CACHE: std::cell::RefCell<Option<(std::time::Instant, Vec<Entry>)>> = const { std::cell::RefCell::new(None) };
}

/// список для вкладки: не чаще раза в три секунды, пока она открыта (рисование не ходит по диску каждый кадр)
pub fn entries() -> Vec<Entry> {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().is_none_or(|(t, _)| t.elapsed().as_secs() >= 3) {
            *c = Some((std::time::Instant::now(), scan_dirs(&user_dir(), &system_dirs(), &desktop())));
        }
        c.as_ref().map(|(_, v)| v.clone()).unwrap_or_default()
    })
}

/// `shot settings:startup`: одни и те же записи на любой машине (картинки и проверка влезания не зависят от неё)
pub fn mock() {
    let e = |file: &str, name: &str, exec: &str, enabled: bool, other: bool, user: bool| Entry { file: file.into(), name: name.into(), exec: exec.into(), enabled, other_desktop: other, user };
    let v = vec![
        e("blueman.desktop", "Blueman Applet", "blueman-applet", true, false, false),
        e("dropbox.desktop", "Dropbox", "dropbox start -i --with-a-very-long-argument-list-that-must-be-cut-somewhere", true, false, true),
        e("nm-applet.desktop", "Network Manager Applet", "nm-applet", false, false, true),
        e("baloo_file.desktop", "Baloo File Daemon", "baloo_file", true, true, false),
    ];
    CACHE.with(|c| *c.borrow_mut() = Some((std::time::Instant::now(), v)));
}

/// переключить i-ю запись списка; список заново читается сразу
pub fn toggle(i: usize) -> bool {
    let list = entries();
    let Some(e) = list.get(i) else { return false };
    let ok = set_enabled(&user_dir(), &system_dirs(), &desktop(), &e.file, !e.enabled);
    CACHE.with(|c| *c.borrow_mut() = None);
    ok
}

/// вкладка: записи автозапуска, что запустится, сверху; не для этого стола — внизу, тусклым
pub fn tab(ui: &mut crate::ui::Ui, st: &crate::app::State, b: crate::gfx::R) {
    use crate::gfx::R;
    use crate::ui::Act;
    let t = ui.t;
    let mut y = crate::settings::title(ui, b.x, b.y, b.w, "программы, которые запускаются при входе");
    let list = entries();
    let _ = st;
    for (i, e) in list.iter().enumerate() {
        if e.other_desktop {
            ui.text_in(nemo_px::font::PX, R::new(b.x + 4, y, b.w - 4, 24), &format!("{}  ·  {}", e.name, crate::i18n::tr("не для этого рабочего стола")), t.text_faint);
            y += 26;
            continue;
        }
        let exec: String = e.exec.split_whitespace().next().unwrap_or("").chars().take(60).collect();
        ui.toggle(R::new(b.x, y, b.w, 40), &e.name, &exec, e.enabled, Act::SStartup(i));
        y += 46;
    }
    if list.is_empty() {
        ui.text_in(nemo_px::font::PX, R::new(b.x, y, b.w, 24), "в автозапуске ничего нет", t.text_faint);
        y += 26;
    }
    ui.text_in(nemo_px::font::PX, R::new(b.x, y + 8, b.w, 24), "выключение — файл-пометка в ~/.config/autostart; убрать его — включить", t.text_faint);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nemo-startup-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    const BLUEMAN: &str = "[Desktop Entry]\nType=Application\nName=Blueman\nExec=blueman-applet\n";

    #[test]
    fn an_entry_is_read_with_its_switches() {
        let e = parse("a.desktop", "[Desktop Entry]\nType=Application\nName=Applet\nExec=applet --x\nHidden=true\n", "niri", false).unwrap();
        assert_eq!((e.name.as_str(), e.exec.as_str(), e.enabled, e.other_desktop), ("Applet", "applet --x", false, false));
        let g = parse("g.desktop", "[Desktop Entry]\nName=G\nX-GNOME-Autostart-enabled=false\n", "niri", false).unwrap();
        assert!(!g.enabled);
        // чужой рабочий стол: OnlyShowIn без нашего или NotShowIn с нашим
        assert!(parse("k.desktop", "[Desktop Entry]\nName=K\nOnlyShowIn=KDE;\n", "niri", false).unwrap().other_desktop);
        assert!(parse("n.desktop", "[Desktop Entry]\nName=N\nNotShowIn=niri;GNOME;\n", "niri", false).unwrap().other_desktop);
        assert!(!parse("o.desktop", "[Desktop Entry]\nName=O\nOnlyShowIn=niri;KDE;\n", "niri", false).unwrap().other_desktop);
        // не программа
        assert!(parse("l.desktop", "[Desktop Entry]\nType=Link\nName=L\n", "niri", false).is_none());
        // имя из имени файла, если Name нет
        assert_eq!(parse("plain.desktop", "[Desktop Entry]\nExec=x\n", "niri", false).unwrap().name, "plain");
    }

    #[test]
    fn a_system_entry_is_turned_off_by_a_marker_and_on_again_by_removing_it() {
        let (user, sys) = (tmp("u1"), tmp("s1"));
        std::fs::write(sys.join("blueman.desktop"), BLUEMAN).unwrap();
        let s = [sys.clone()];
        assert!(scan_dirs(&user, &s, "niri")[0].enabled);
        assert!(set_enabled(&user, &s, "niri", "blueman.desktop", false));
        let e = &scan_dirs(&user, &s, "niri")[0];
        assert!(!e.enabled && e.user);
        let marker = std::fs::read_to_string(user.join("blueman.desktop")).unwrap();
        assert!(marker.starts_with(MARK) && marker.contains("Hidden=true"));
        // системный файл не тронут
        assert_eq!(std::fs::read_to_string(sys.join("blueman.desktop")).unwrap(), BLUEMAN);
        assert!(set_enabled(&user, &s, "niri", "blueman.desktop", true));
        assert!(!user.join("blueman.desktop").exists());
        assert!(scan_dirs(&user, &s, "niri")[0].enabled);
    }

    /// свой файл человека не стирается: выключение и включение правят две строки, остальное остаётся
    #[test]
    fn a_users_own_file_is_edited_not_deleted() {
        let (user, sys) = (tmp("u2"), tmp("s2"));
        let own = "# my note\n[Desktop Entry]\nType=Application\nName=Mine\nExec=mine\nComment=keep me\n";
        std::fs::write(user.join("mine.desktop"), own).unwrap();
        assert!(set_enabled(&user, &[sys.clone()], "niri", "mine.desktop", false));
        let off = std::fs::read_to_string(user.join("mine.desktop")).unwrap();
        assert!(off.contains("Hidden=true") && off.contains("Comment=keep me") && off.contains("# my note"));
        assert!(!scan_dirs(&user, &[sys.clone()], "niri")[0].enabled);
        assert!(set_enabled(&user, &[sys.clone()], "niri", "mine.desktop", true));
        let on = std::fs::read_to_string(user.join("mine.desktop")).unwrap();
        assert!(!on.contains("Hidden") && on.contains("Comment=keep me") && on.contains("Exec=mine"));
    }

    #[test]
    fn user_files_win_names_are_safe_and_other_desktops_go_last() {
        let (user, sys) = (tmp("u3"), tmp("s3"));
        std::fs::write(sys.join("a.desktop"), "[Desktop Entry]\nName=Zed System\nExec=z\n").unwrap();
        std::fs::write(user.join("a.desktop"), "[Desktop Entry]\nName=Zed User\nExec=z2\n").unwrap();
        std::fs::write(sys.join("b.desktop"), "[Desktop Entry]\nName=Alpha\nExec=a\nOnlyShowIn=KDE;\n").unwrap();
        std::fs::write(sys.join("c.desktop"), "[Desktop Entry]\nName=Beta\nExec=b\n").unwrap();
        let l = scan_dirs(&user, &[sys.clone()], "niri");
        let names: Vec<&str> = l.iter().map(|e| e.name.as_str()).collect();
        // Beta, Zed User (пользователь перекрыл системную), потом чужой стол — Alpha
        assert_eq!(names, ["Beta", "Zed User", "Alpha"]);
        assert!(l.iter().any(|e| e.name == "Zed User" && e.user));
        // имя файла — не путь и не скрытый файл
        assert!(!set_enabled(&user, &[sys.clone()], "niri", "../evil.desktop", false));
        assert!(!set_enabled(&user, &[sys], "niri", ".hidden.desktop", false));
    }
}
