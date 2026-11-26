// Модули бара из скриптов, как custom-модули waybar: исполняемый файл
// ~/.config/nemo/scripts/<имя> становится модулем `x:<имя>` (он есть в настройках → бар).
// Скрипт печатает строку на каждое обновление — просто текст или JSON:
//   {"text": "21°", "icon": "temp", "color": "accent", "click": "xdg-open https://…", "interval": 600}
// icon — любой значок 7×7 шелла (список в docs/CONFIG.md), color — роль темы (accent, bright, ok,
// warn, text, dim). Скрипт, что работает постоянно, шлёт обновления строками; завершившийся
// запускается снова через `interval` секунд (по умолчанию 60; раньше 10 — лишний запуск оболочки и утилит шесть раз в минуту). Каждый модуль — одна нить и одна
// группа процессов, и только пока он в баре; убирается из бара или падает шелл — группа
// получает SIGTERM целиком (вместе с тем, что скрипт запустил сам: см. LEADER).
// i18n: inline pairs

use crate::app::Msg;
use calloop::channel::Sender;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// что показывает модуль
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plug {
    pub text: String,
    pub icon: String,
    pub color: String,
    pub click: String,
}

/// SIGTERM группе процессов скрипта (pid = id группы: скрипт — её лидер)
fn term(pid: &AtomicU32) {
    let p = pid.load(Ordering::SeqCst);
    if p > 0 {
        unsafe { libc::kill(-(p as i32), libc::SIGTERM) };
    }
}

/// работающий модуль: флаг остановки и pid текущего запуска скрипта
struct Run {
    stop: Arc<AtomicBool>,
    pid: Arc<AtomicU32>,
    /// нить кончила работу (скрипт отключён за сбои, не запускается): sync может поднять её заново, когда его включили
    done: Arc<AtomicBool>,
}

/// безопасный режим: столько быстрых сбоев подряд — и скрипт выключается, пока его не включат вручную (`nemo scripts reset ИМЯ`)
pub const CRASH_LIMIT: u32 = 5;
/// быстрый — упал раньше, чем за столько секунд
pub const FAST_SECS: f32 = 3.0;

/// запуск считается сбоем: недолго прожил и кончил с ошибкой
pub fn is_crash(lived_secs: f32, success: bool) -> bool {
    lived_secs < FAST_SECS && !success
}

fn off_file() -> PathBuf {
    crate::config::state_dir().join("scripts-off.json")
}

/// скрипты, выключенные безопасным режимом
pub fn disabled() -> Vec<String> {
    std::fs::read_to_string(off_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn set_disabled(name: &str, off: bool) {
    let mut v = disabled();
    v.retain(|n| n != name);
    if off {
        v.push(name.to_string());
        v.sort();
    }
    let _ = crate::config::write_atomic(&off_file(), &serde_json::to_string(&v).unwrap_or_default());
}

#[derive(Default)]
pub struct Plugins {
    pub shown: HashMap<String, Plug>,
    runs: HashMap<String, Run>,
}

pub fn dir() -> PathBuf {
    crate::dirs::scripts_dir()
}

/// имя модуля — одно имя файла в папке модулей: без «/», «..» и пробелов (из настроек или
/// чужой темы пришёл бы путь к любой программе на диске)
pub fn name_ok(n: &str) -> bool {
    !n.is_empty() && n.len() <= 64 && !n.starts_with('.') && n.chars().all(|c| c.is_alphanumeric() || "._-".contains(c))
}

/// исполняемые файлы в папке модулей → id модулей `x:<имя>`
pub fn available() -> Vec<String> {
    scripts().into_iter().map(|n| format!("x:{n}")).collect()
}

/// имена скриптов в ~/.config/nemo/scripts (исполняемые файлы), по алфавиту: для виджета «скрипт»
pub fn scripts() -> Vec<String> {
    use std::os::unix::fs::PermissionsExt;
    let mut v: Vec<String> = std::fs::read_dir(dir())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false))
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .filter(|n| name_ok(n))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// строка вывода → что показать; обычный текст — сам текст
pub fn parse(line: &str) -> (Plug, Option<u64>) {
    let line = line.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
        if v.is_object() {
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            // виджету — несколько строк; бар берёт первую (bar.rs)
            let text: String = s("text").chars().take(400).collect();
            return (Plug { text, icon: s("icon"), color: s("color"), click: s("click") }, v.get("interval").and_then(|x| x.as_u64()));
        }
    }
    (Plug { text: line.chars().take(48).collect(), ..Default::default() }, None)
}

impl Plugins {
    /// запустить модули, что в баре (и виджеты-скрипты), остановить те, что ушли
    pub fn sync(&mut self, on_bar: &[String], tx: &Sender<Msg>) {
        let want: Vec<&String> = on_bar.iter().filter(|m| m.strip_prefix("x:").is_some_and(name_ok)).collect();
        let gone: Vec<String> = self.runs.keys().filter(|k| !want.contains(k)).cloned().collect();
        for k in gone {
            if let Some(r) = self.runs.remove(&k) {
                r.stop.store(true, Ordering::SeqCst);
                term(&r.pid);
            }
            self.shown.remove(&k);
        }
        for m in want {
            if let Some(r) = self.runs.get(m) {
                // нить вышла (отключён за сбои), а его включили обратно — запустить заново
                if r.done.load(Ordering::SeqCst) && !disabled().iter().any(|n| Some(n.as_str()) == m.strip_prefix("x:")) {
                    self.runs.remove(m);
                } else {
                    continue;
                }
            }
            let run = Run { stop: Arc::new(AtomicBool::new(false)), pid: Arc::new(AtomicU32::new(0)), done: Arc::new(AtomicBool::new(false)) };
            spawn(m.clone(), run.stop.clone(), run.pid.clone(), run.done.clone(), tx.clone());
            self.runs.insert(m.clone(), run);
        }
    }
}

impl Drop for Plugins {
    fn drop(&mut self) {
        for r in self.runs.values() {
            r.stop.store(true, Ordering::SeqCst);
            term(&r.pid);
        }
    }
}

/// лидер группы скрипта — маленький sh: сигнал о смерти шелла (PDEATHSIG) достаётся только ему, и
/// он передаёт SIGTERM всей группе. Самому скрипту и тому, что он запустил в фоне, этот сигнал не
/// пришёл бы (проверено kill -9 шелла: без лидера `sleep 999 &` из скрипта оставался сиротой).
/// Скрипт — "$0"
const LEADER: &str = r#"trap 'trap - TERM; kill -TERM 0' TERM; "$0" & wait $!"#;

fn spawn(id: String, stop: Arc<AtomicBool>, pid: Arc<AtomicU32>, done: Arc<AtomicBool>, tx: Sender<Msg>) {
    use std::os::unix::fs::PermissionsExt;
    let path = dir().join(&id[2..]);
    std::thread::spawn(move || {
        let mut wait = 60u64;
        let mut crashes = 0u32;
        let off = Plug { text: "!".into(), icon: "close".into(), color: "warn".into(), click: String::new() };
        // выключен безопасным режимом раньше: молчит, значок «!» вместо вывода
        if disabled().iter().any(|n| n == &id[2..]) {
            let _ = tx.send(Msg::Plugin(id.clone(), off));
            done.store(true, Ordering::SeqCst);
            return;
        }
        while !stop.load(Ordering::SeqCst) {
            let started = std::time::Instant::now();
            // не исполняемый — значок ошибки в баре (запуск через sh сам бы не упал)
            let runnable = std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0);
            let mut cmd = Command::new("/bin/sh");
            cmd.arg("-c").arg(LEADER).arg(&path).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).process_group(0);
            // шелл упал или перезапустился — лидер получает SIGTERM и гасит группу
            crate::proc::owned(&mut cmd);
            let child = if runnable { cmd.spawn().ok() } else { None };
            let Some(mut child) = child else {
                let _ = tx.send(Msg::Plugin(id.clone(), Plug { text: "?".into(), icon: "close".into(), color: "warn".into(), click: String::new() }));
                done.store(true, Ordering::SeqCst);
                return;
            };
            pid.store(child.id(), Ordering::SeqCst);
            // убрали из бара, пока запускался: sync ещё не знал pid
            if stop.load(Ordering::SeqCst) {
                term(&pid);
            }
            if let Some(out) = child.stdout.take() {
                let mut rd = BufReader::new(out);
                let mut buf = Vec::with_capacity(256);
                loop {
                    // строка длиннее 4 КБ — не дело модуля бара: дочитывается и пропускается
                    buf.clear();
                    match (&mut rd).take(4097).read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    if buf.len() > 4096 && buf.last() != Some(&b'\n') {
                        while matches!((&mut rd).take(4096).read_until(b'\n', &mut buf), Ok(n) if n > 0) && buf.last() != Some(&b'\n') {
                            buf.clear();
                        }
                        continue;
                    }
                    let line = String::from_utf8_lossy(&buf);
                    if line.trim().is_empty() {
                        continue;
                    }
                    let (p, iv) = parse(&line);
                    if let Some(iv) = iv {
                        wait = iv.clamp(1, 86_400);
                    }
                    if tx.send(Msg::Plugin(id.clone(), p)).is_err() {
                        term(&pid);
                        let _ = child.wait();
                        return;
                    }
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
            term(&pid);
            let _ = child.kill();
            let status = child.wait();
            pid.store(0, Ordering::SeqCst);
            // быстрые сбои подряд: не крутить сломанный скрипт каждую секунду — выключить и сказать
            if !stop.load(Ordering::SeqCst) {
                crashes = if is_crash(started.elapsed().as_secs_f32(), status.as_ref().is_ok_and(|s| s.success())) { crashes + 1 } else { 0 };
                if crashes >= CRASH_LIMIT {
                    set_disabled(&id[2..], true);
                    let _ = tx.send(Msg::Plugin(id.clone(), off));
                    done.store(true, Ordering::SeqCst);
                    return;
                }
            }
            // спим короткими шагами: убрали модуль из бара — он быстро останавливается
            for _ in 0..wait * 4 {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
    });
}

/// цвет темы по имени роли
pub fn color(t: &crate::theme::Theme, role: &str) -> crate::gfx::C {
    match role {
        "accent" => t.red,
        "bright" => t.ember,
        "ok" => t.ok,
        "warn" => t.warn,
        "dim" => t.text_dim,
        _ => t.text,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lines() {
        let (p, iv) = super::parse(r#"{"text":"21°","icon":"temp","color":"accent","interval":600}"#);
        assert_eq!((p.text.as_str(), p.icon.as_str(), p.color.as_str(), iv), ("21°", "temp", "accent", Some(600)));
        let (p, iv) = super::parse("  hello  ");
        assert_eq!((p.text.as_str(), iv), ("hello", None));
        assert!(super::name_ok("weather.sh") && !super::name_ok("../../bin/sh") && !super::name_ok("/usr/bin/x") && !super::name_ok(".hidden"));
    }
}

#[cfg(test)]
mod safe_mode_tests {
    #[test]
    fn only_a_quick_failure_counts_as_a_crash() {
        assert!(super::is_crash(0.2, false));
        assert!(!super::is_crash(0.2, true), "a quick success is a script that simply finished");
        assert!(!super::is_crash(30.0, false), "a failure after a long run is not a crash loop");
        assert!(super::CRASH_LIMIT >= 3, "one or two failures must not switch a script off");
    }
}

#[cfg(test)]
mod run_tests {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    /// скрипт и то, что он запустил в фоне, уходят вместе: когда модуль убрали из бара (SIGTERM
    /// группе) и когда шелл упал (PDEATHSIG приходит только лидеру группы — он гасит остальных)
    #[test]
    fn stops_with_its_children() {
        let d = std::env::temp_dir().join(format!("nemo-plug-test-{}", std::process::id()));
        let m = d.join("nemo/scripts");
        std::fs::create_dir_all(&m).unwrap();
        let marker = d.join("child.pid");
        let script = m.join("t.sh");
        std::fs::write(&script, format!("#!/bin/sh\nsleep 300 &\necho $! > {}\necho hi\nwait\n", marker.display())).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(m.join("noexec"), "#!/bin/sh\necho x\n").unwrap();
        // SAFETY: один тест меняет XDG_CONFIG_HOME; другие тесты его не читают
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &d) };
        let first = |name: &str| {
            let (tx, rx) = calloop::channel::channel();
            let (stop, pid) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicU32::new(0)));
            let _ = std::fs::remove_file(&marker);
            super::spawn(format!("x:{name}"), stop.clone(), pid.clone(), Arc::new(AtomicBool::new(false)), tx);
            let t0 = std::time::Instant::now();
            loop {
                if let Ok(crate::app::Msg::Plugin(_, p)) = rx.try_recv() {
                    return (p, stop, pid);
                }
                assert!(t0.elapsed().as_secs() < 5, "no output");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };
        let gone = |child: i32, what: &str| {
            let t0 = std::time::Instant::now();
            while std::fs::read_to_string(format!("/proc/{child}/stat")).map(|s| !s.contains(") Z")).unwrap_or(false) {
                assert!(t0.elapsed().as_secs() < 5, "the script's child survived: {what}");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };
        let child = || -> i32 {
            let t0 = std::time::Instant::now();
            loop {
                if let Some(c) = std::fs::read_to_string(&marker).ok().and_then(|t| t.trim().parse().ok()) {
                    return c;
                }
                assert!(t0.elapsed().as_secs() < 5, "no child pid");
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        };
        // убрали из бара
        let (got, stop, pid) = first("t.sh");
        assert_eq!(got.text, "hi");
        let c = child();
        assert!(std::path::Path::new(&format!("/proc/{c}")).exists());
        stop.store(true, Ordering::SeqCst);
        super::term(&pid);
        gone(c, "removed from the bar");
        // шелл упал: SIGTERM только лидеру, как от PDEATHSIG
        let (_, stop, pid) = first("t.sh");
        let c = child();
        stop.store(true, Ordering::SeqCst);
        unsafe { libc::kill(pid.load(Ordering::SeqCst) as i32, libc::SIGTERM) };
        gone(c, "the shell crashed");
        // не исполняемый — значок ошибки
        let (got, _, _) = first("noexec");
        assert_eq!(got.icon, "close");
        // скрипт, что печатает и тут же падает с интервалом в секунду: после CRASH_LIMIT сбоев его выключают
        unsafe { std::env::set_var("XDG_STATE_HOME", &d) };
        let crash = m.join("crash.sh");
        std::fs::write(&crash, "#!/bin/sh\necho '{\"text\":\"x\",\"interval\":1}'\nexit 1\n").unwrap();
        std::fs::set_permissions(&crash, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (tx, rx) = calloop::channel::channel();
        let (stop, pid, done) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
        super::spawn("x:crash.sh".into(), stop.clone(), pid, done.clone(), tx);
        let t0 = std::time::Instant::now();
        while !done.load(Ordering::SeqCst) {
            assert!(t0.elapsed().as_secs() < 20, "the crashing script was never switched off");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(super::disabled().contains(&"crash.sh".to_string()));
        let mut last = None;
        while let Ok(crate::app::Msg::Plugin(_, p)) = rx.try_recv() {
            last = Some(p);
        }
        assert_eq!(last.map(|p| p.text), Some("!".to_string()), "the bar shows a warning mark in its place");
        // выключенный не запускается снова, пока его не включили
        let (tx2, rx2) = calloop::channel::channel();
        let done2 = Arc::new(AtomicBool::new(false));
        super::spawn("x:crash.sh".into(), Arc::new(AtomicBool::new(false)), Arc::new(AtomicU32::new(0)), done2.clone(), tx2);
        let t1 = std::time::Instant::now();
        while !done2.load(Ordering::SeqCst) {
            assert!(t1.elapsed().as_secs() < 5);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(matches!(rx2.try_recv(), Ok(crate::app::Msg::Plugin(_, p)) if p.text == "!"));
        super::set_disabled("crash.sh", false);
        assert!(!super::disabled().contains(&"crash.sh".to_string()), "reset turns it back on");
        let _ = std::fs::remove_dir_all(&d);
    }
}
