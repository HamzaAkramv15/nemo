// Вкладка «состояние»: здоровье рабочего стола без терминала. Чего не хватает из программ, что запущено и сколько памяти
// занимает, версии, как идут обновления и были ли решения «поставить вопреки подписи». Данные собираются при открытии
// вкладки (collect), а не в каждом кадре; кнопки — обновить, открыть папку настроек и состояния, создать отчёт для issue.

use crate::app::State;
use crate::font::PX;
use crate::gfx::R;
use crate::settings::title;
use crate::ui::{Act, Tone, Ui};

pub struct Diag {
    /// запущенные части Nemo: имя, память (PSS, МБ), потоков
    pub procs: Vec<(String, f32, u32)>,
    /// сколько программ из списка есть и каких нет: (команда, зачем, нужна ли обязательно)
    pub present: usize,
    pub missing: Vec<(&'static str, &'static str, bool)>,
    pub niri: String,
    /// решений «поставить вопреки подписи» в журнале и последнее из них
    pub overrides: usize,
    pub last_override: String,
}

fn threads(pid: u32) -> u32 {
    std::fs::read_to_string(format!("/proc/{pid}/status")).ok().and_then(|t| crate::sys::kv_num(&t, "Threads:")).unwrap_or(0.0) as u32
}

/// из журнала решений: сколько строк и последняя, читаемо («время версия подпись»)
pub fn parse_overrides(text: &str) -> (usize, String) {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let last = lines.last().map(|l| {
        let f: Vec<&str> = l.split('\t').collect();
        let get = |k: &str| f.iter().find_map(|x| x.strip_prefix(k)).unwrap_or("?");
        let at: u64 = f.first().and_then(|a| a.parse().ok()).unwrap_or(0);
        let tm = nemo_px::time::local_tm(at as i64);
        format!("{:04}-{:02}-{:02} {:02}:{:02} · {} · {}", tm.year, tm.mon, tm.day, tm.hour, tm.min, get("version="), get("signature="))
    });
    (lines.len(), last.unwrap_or_default())
}

pub fn collect() -> Diag {
    let mut procs = vec![];
    if let Some(pid) = crate::doctor::shell_pids().first() {
        procs.push(("nemo-shell".to_string(), crate::doctor::pss_mb(*pid), threads(*pid)));
    }
    for (name, arg) in [("nemo-agents", ""), ("nemo-files", "--portal")] {
        if let Some(pid) = nemo_px::paths::pids(name, arg).first() {
            procs.push((if arg.is_empty() { name.to_string() } else { format!("{name} {arg}") }, crate::doctor::pss_mb(*pid), threads(*pid)));
        }
    }
    let (mut present, mut missing) = (0, vec![]);
    for (cmd, ru, _en, required) in crate::doctor::DEPS {
        if crate::doctor::which(cmd) {
            present += 1;
        } else {
            missing.push((*cmd, *ru, *required));
        }
    }
    let niri = std::process::Command::new("niri").arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let (overrides, last_override) = std::fs::read_to_string(crate::upinstall::overrides_path()).map(|t| parse_overrides(&t)).unwrap_or_default();
    Diag { procs, present, missing, niri, overrides, last_override }
}

pub(crate) fn tab_diag(ui: &mut Ui, st: &State, b: R) {
    let t = ui.t;
    let Some(d) = &st.diag else {
        ui.text_in(PX, R::new(b.x, b.y, b.w, 22), "собираю…", t.text_dim);
        return;
    };
    let half = (b.w - 12) / 2;
    let mut y = title(ui, b.x, b.y, b.w, "запущено сейчас");
    for (name, mb, th) in &d.procs {
        ui.text_in(PX, R::new(b.x, y, 220, 22), name, t.text);
        ui.text_in(PX, R::new(b.x + 230, y, b.w - 230, 22), &crate::tf!("{:.1} МБ памяти · потоков {}", "{:.1} MB of memory · {} threads", "{:.1} MB de memória · {} threads", mb, th), t.text_dim);
        y += 24;
    }
    if d.procs.is_empty() {
        ui.text_in(PX, R::new(b.x, y, b.w, 22), "ни одной части Nemo не нашёл в процессах", t.warn);
        y += 24;
    }
    y += 8;
    y = title(ui, b.x, y, b.w, "программы, которым Nemo доверяет работу");
    let need = d.missing.iter().filter(|m| m.2).count();
    let line = if d.missing.is_empty() {
        crate::tf!("все {} на месте", "all {} are here", "todos os {} estão presentes", d.present)
    } else {
        crate::tf!("{} из {} на месте, не хватает {}", "{} of {} are here, {} missing", "{} de {} presentes, faltam {}", d.present, d.present + d.missing.len(), d.missing.len())
    };
    ui.text_in(PX, R::new(b.x, y, b.w, 22), &line, if need > 0 { t.warn } else if d.missing.is_empty() { t.ok } else { t.text_dim });
    y += 26;
    for (cmd, why, required) in d.missing.iter().take(8).map(|(c, ru, req)| (c, if crate::i18n::translated() { crate::doctor::DEPS.iter().find(|x| x.0 == *c).map_or(*ru, |x| x.2) } else { *ru }, req)) {
        ui.cv.fill(R::new(b.x + 2, y + 8, 5, 5), if *required { t.warn } else { t.text_faint });
        ui.text_in(PX, R::new(b.x + 14, y, 150, 20), cmd, t.text);
        ui.text_in(PX, R::new(b.x + 170, y, b.w - 170, 20), why, t.text_dim);
        if *required {
            let off = ui.w(PX, why) + 24;
            ui.text_in(PX, R::new(b.x + 170 + off, y, 200, 20), "без неё не работает основное", t.warn);
        }
        y += 22;
    }
    if !d.missing.is_empty() {
        ui.text_in(PX, R::new(b.x, y + 2, b.w, 20), "как поставить: nemo doctor", t.text_faint);
        y += 26;
    }
    y += 8;
    y = title(ui, b.x, y, b.w, "версии");
    let u = &st.upd.stamp;
    let ver = if u.version.is_empty() { env!("CARGO_PKG_VERSION").to_string() } else { u.version.clone() };
    let commit: String = u.commit.chars().take(8).collect();
    for (k, v) in [
        ("nemo", crate::tf!("{} (шелл {}{})", "{} (shell {}{})", "{} (shell {}{})", nemo_px::brand::VERSION, ver, if commit.is_empty() { String::new() } else { format!(", {commit}") })),
        ("niri", if d.niri.is_empty() { "—".to_string() } else { format!("{}{}", d.niri, if st.niri.nemo { " · nemo-niri" } else { "" }) }),
    ] {
        ui.text_in(PX, R::new(b.x, y, 120, 22), k, t.text_faint);
        ui.text_in(PX, R::new(b.x + 130, y, b.w - 130, 22), &v, t.text);
        y += 24;
    }
    y += 8;
    y = title(ui, b.x, y, b.w, "проверка обновлений");
    let policy = match st.set.update_verify.as_str() {
        "require" => "всегда: без подписи не ставлю",
        "off" => "выключена: подпись не проверяется",
        _ => "по ключам: проверяю, когда ключи есть",
    };
    ui.text_in(PX, R::new(b.x, y, 150, 22), "проверка подписей", t.text_faint);
    ui.text_in(PX, R::new(b.x + 160, y, b.w - 160, 22), policy, if st.set.update_verify == "off" { t.warn } else { t.text });
    y += 24;
    ui.text_in(PX, R::new(b.x, y, 150, 22), "без подписи", t.text_faint);
    let (txt, col) = if d.overrides == 0 {
        (crate::i18n::tr("не было").to_string(), t.text)
    } else {
        (crate::tf!("{}; последнее: {}", "{}; the last: {}", "{}; a última: {}", d.overrides, d.last_override), t.warn)
    };
    ui.text_in(PX, R::new(b.x + 160, y, b.w - 160, 22), &txt, col);
    y += 34;
    ui.button(R::new(b.x, y, half, 28), "обновить сведения", "reboot", Tone::Plain, false, true, Act::SCycle("diag_refresh"));
    ui.button(R::new(b.x + half + 12, y, half, 28), "создать отчёт для issue", "save", Tone::Primary, false, true, Act::SCycle("diag_report"));
    y += 36;
    ui.button(R::new(b.x, y, half, 28), "папка настроек", "folder", Tone::Plain, false, true, Act::SCycle("diag_open_cfg"));
    ui.button(R::new(b.x + half + 12, y, half, 28), "папка состояния и журналов", "folder", Tone::Plain, false, true, Act::SCycle("diag_open_state"));
    y += 38;
    ui.text_in(PX, R::new(b.x, y, b.w, 20), "отчёт: архив в домашней папке, паролей в нём нет", t.text_faint);
    ui.cv.reach = ui.cv.reach.max(y + 24);
}

#[cfg(test)]
mod tests {
    use super::parse_overrides;

    #[test]
    fn the_decision_log_is_counted_and_its_last_line_made_readable() {
        let t = "1700000000\tversion=3.2.0\tcommit=abc\tsignature=unsigned\tpolicy=require\tdecided=apply --force\n\n1700005000\tversion=3.2.1\tcommit=def\tsignature=bad\tpolicy=auto\tdecided=to --force\n";
        let (n, last) = parse_overrides(t);
        assert_eq!(n, 2);
        assert!(last.contains("3.2.1") && last.contains("bad"), "{last}");
        assert_eq!(parse_overrides(""), (0, String::new()));
        let (n, last) = parse_overrides("garbage line\n");
        assert_eq!(n, 1);
        assert!(last.contains('?'), "{last}");
    }
}
