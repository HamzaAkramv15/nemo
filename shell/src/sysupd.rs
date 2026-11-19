// Обновления самого дистрибутива в настройках: сколько пакетов ждёт обновления и кнопка «обновить в
// терминале» (sudo спросит пароль там, шелл его не видит). Fedora — полноценно: `nemo-fedora-update`
// делит обновления на безопасные и удержанные (ядро, графика, основа системы); Arch, Debian/Ubuntu и
// openSUSE — число ждущих пакетов и обычная команда обновления. Проверка идёт в отдельной нити и только
// когда страница открыта (не чаще раза в шесть часов): ни таймера, ни фоновых запросов.

use crate::app::{Msg, State};
use crate::font::PX;
use crate::gfx::R;
use crate::settings::title;
use crate::ui::{Act, Tone, Ui};
use std::process::{Command, Stdio};
use calloop::channel::Sender;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Family {
    Fedora,
    Arch,
    Debian,
    Suse,
    Other,
}

/// дистрибутив по /etc/os-release: (семейство, название для показа)
pub fn parse_os_release(text: &str) -> (Family, String) {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(&format!("{k}=")).map(|v| v.trim().trim_matches('"').to_string())).unwrap_or_default();
    let (id, like, name) = (get("ID"), get("ID_LIKE"), get("PRETTY_NAME"));
    let has = |w: &str| id == w || like.split_whitespace().any(|x| x == w);
    let fam = if has("fedora") || has("rhel") {
        Family::Fedora
    } else if has("arch") {
        Family::Arch
    } else if has("debian") || has("ubuntu") {
        Family::Debian
    } else if has("suse") || has("opensuse") {
        Family::Suse
    } else {
        Family::Other
    };
    (fam, if name.is_empty() { id } else { name })
}

pub fn detect() -> (Family, String) {
    parse_os_release(&std::fs::read_to_string("/etc/os-release").unwrap_or_default())
}

/// что показать после проверки
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Found {
    /// всего ждёт обновления
    pub total: usize,
    /// из них безопасных; None — дистрибутив не делит (все равны)
    pub safe: Option<usize>,
    /// удержано по причинам: (причина, сколько)
    pub held: Vec<(String, usize)>,
    pub err: String,
}

#[derive(Default)]
pub struct Sys {
    pub fam: Option<(Family, String)>,
    pub found: Option<Found>,
    pub checking: bool,
    /// когда последний раз проверяли (секунды с 1970)
    pub at: u64,
}

/// ответ `nemo-fedora-update --json`
pub fn parse_fedora_json(text: &str) -> Option<Found> {
    let v: serde_json::Value = serde_json::from_str(text.lines().rev().find(|l| l.trim_start().starts_with('{'))?).ok()?;
    let mut held: Vec<(String, usize)> = v["held_by"].as_object()?.iter().map(|(k, n)| (k.clone(), n.as_u64().unwrap_or(0) as usize)).collect();
    held.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Some(Found { total: v["total"].as_u64()? as usize, safe: Some(v["safe"].as_u64()? as usize), held, err: String::new() })
}

/// сколько непустых строк вывода — столько пакетов (Arch: checkupdates; Debian: строки «Inst»; SUSE: строки таблицы)
pub fn count_lines(family: Family, out: &str) -> usize {
    match family {
        Family::Arch => out.lines().filter(|l| !l.trim().is_empty()).count(),
        Family::Debian => out.lines().filter(|l| l.starts_with("Inst ")).count(),
        // zypper lu: «v | repo | имя | …», шапка и разделители не считаются
        Family::Suse => out.lines().filter(|l| l.starts_with("v ") || l.starts_with("v|")).count(),
        _ => 0,
    }
}

fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
    let o = Command::new(cmd).args(args).stdin(Stdio::null()).output().map_err(|e| format!("{cmd}: {e}"))?;
    Ok(String::from_utf8_lossy(&o.stdout).to_string())
}

/// проверка; блокирует нить, зови из отдельной
pub fn check(family: Family) -> Found {
    let fail = |e: String| Found { err: e, ..Default::default() };
    match family {
        Family::Fedora => {
            let exe = yami_px::paths::find_exe("nemo-fedora-update");
            match exe.as_ref().map(|e| run(&e.to_string_lossy(), &["--json"])) {
                Some(Ok(t)) => parse_fedora_json(&t).unwrap_or_else(|| fail(crate::tf!("не разобрал ответ nemo-fedora-update", "could not read nemo-fedora-update's answer", "não entendi a resposta do nemo-fedora-update"))),
                Some(Err(e)) => fail(e),
                None => match run("dnf", &["repoquery", "--upgrades", "--qf", "%{name}\n"]) {
                    Ok(t) => Found { total: t.lines().filter(|l| !l.trim().is_empty() && !l.contains("repositories") && !l.starts_with(' ')).count(), ..Default::default() },
                    Err(e) => fail(e),
                },
            }
        }
        Family::Arch => match run("checkupdates", &[]) {
            Ok(t) => Found { total: count_lines(family, &t), ..Default::default() },
            Err(_) => fail(crate::tf!("нужен checkupdates (пакет pacman-contrib)", "needs checkupdates (package pacman-contrib)", "precisa do checkupdates (pacote pacman-contrib)")),
        },
        Family::Debian => match run("apt-get", &["-s", "upgrade"]) {
            Ok(t) => Found { total: count_lines(family, &t), ..Default::default() },
            Err(e) => fail(e),
        },
        Family::Suse => match run("zypper", &["--non-interactive", "--quiet", "list-updates"]) {
            Ok(t) => Found { total: count_lines(family, &t), ..Default::default() },
            Err(e) => fail(e),
        },
        Family::Other => fail(crate::tf!("для этого дистрибутива пока нет проверки: обновляй его менеджером пакетов", "no check for this distribution yet: update it with its package manager", "ainda sem verificação para esta distribuição: atualize com o gerenciador de pacotes")),
    }
}

/// причина удержания (английский текст от nemo-fedora-update) на языке интерфейса; скачок версии — как есть
fn reason_label(en: &str) -> String {
    let ru = match en {
        "boot and kernel" => "загрузка и ядро",
        "graphics drivers" => "видеодрайверы",
        "the base of the system" => "основа системы",
        "our own session" => "твой сеанс",
        "held by you" => "удержано тобой",
        _ => return en.to_string(),
    };
    crate::i18n::tr(ru).to_string()
}

/// команда обновления для терминала; None — нет
pub fn update_command(family: Family) -> Option<&'static str> {
    match family {
        Family::Fedora => Some("nemo-fedora-update --apply"),
        Family::Arch => Some("sudo pacman -Syu"),
        Family::Debian => Some("sudo apt update && sudo apt upgrade"),
        Family::Suse => Some("sudo zypper refresh && sudo zypper update"),
        Family::Other => None,
    }
}

/// запустить проверку в нити, если пора (page открыта, не чаще раза в шесть часов)
pub fn start(st: &mut State, tx: Sender<Msg>, force: bool) {
    let now = yami_px::time::unix_now();
    let s = &mut st.sys_upd;
    let (fam, _) = s.fam.get_or_insert_with(detect).clone();
    if s.checking || (!force && s.found.is_some() && now.saturating_sub(s.at) < 6 * 3600) {
        return;
    }
    s.checking = true;
    std::thread::spawn(move || {
        let _ = tx.send(Msg::SysUpd(check(fam)));
    });
}

pub fn tab(ui: &mut Ui, st: &State, b: R) {
    let t = ui.t;
    let s = &st.sys_upd;
    let (fam, name) = s.fam.clone().unwrap_or_else(detect);
    let mut y = title(ui, b.x, b.y, b.w, "обновления системы");
    ui.text_in(PX, R::new(b.x, y, 160, 22), "дистрибутив", t.text_faint);
    ui.text_in(PX, R::new(b.x + 170, y, b.w - 170, 22), &name, t.text);
    y += 30;
    let (line, col) = if s.checking {
        (crate::tf!("проверяю…", "checking…", "verificando…"), t.text_dim)
    } else if let Some(f) = &s.found {
        if !f.err.is_empty() {
            (f.err.clone(), t.warn)
        } else if f.total == 0 {
            (crate::tf!("всё свежее", "everything is up to date", "tudo atualizado"), t.ok)
        } else if let Some(safe) = f.safe {
            (crate::tf!("{} безопасных обновлений · {} удержано", "{} safe updates · {} held back", "{} atualizações seguras · {} retidas", safe, f.total - safe), t.text)
        } else {
            (crate::tf!("ждут обновления: {}", "updates waiting: {}", "atualizações pendentes: {}", f.total), t.text)
        }
    } else {
        (crate::tf!("ещё не проверялось", "not checked yet", "ainda não verificado"), t.text_dim)
    };
    ui.cv.fill(R::new(b.x, y + 7, 6, 6), col);
    ui.text_in(PX, R::new(b.x + 14, y, b.w - 14, 20), &line, col);
    y += 34;
    let half = (b.w - 20) / 2;
    ui.button(R::new(b.x, y, half, 30), "проверить", "reboot", Tone::Plain, false, !s.checking, Act::SCycle("sysupd_check"));
    let can = update_command(fam).is_some() && s.found.as_ref().is_some_and(|f| f.err.is_empty() && f.total > 0);
    let label = if fam == Family::Fedora { "обновить безопасное в терминале" } else { "обновить в терминале" };
    ui.button(R::new(b.x + half + 20, y, half, 30), label, "down", Tone::Primary, false, can, Act::SCycle("sysupd_run"));
    y += 44;
    if let Some(f) = &s.found {
        if !f.held.is_empty() {
            y = title(ui, b.x, y, b.w, "удержано до тебя: ядро, графика, основа системы");
            for (why, n) in &f.held {
                ui.text_in(PX, R::new(b.x, y, b.w - 60, 22), &reason_label(why), t.text_dim);
                ui.text_in(PX, R::new(b.right() - 50, y, 50, 22), &n.to_string(), t.text);
                y += 24;
            }
            y += 6;
            ui.text_in(PX, R::new(b.x, y, b.w, 20), "их ставишь сам, когда можно перезагрузиться: nemo-fedora-update --held", t.text_faint);
        }
    }
    if fam == Family::Fedora && s.found.as_ref().is_some_and(|f| f.safe.is_none()) {
        ui.text_in(PX, R::new(b.x, y, b.w, 20), "nemo-fedora-update не найден: считаю все обновления одинаковыми", t.text_faint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_distribution_is_read_from_os_release() {
        let f = |t: &str| parse_os_release(t).0;
        assert_eq!(f("ID=fedora\nPRETTY_NAME=\"Fedora Linux 45\""), Family::Fedora);
        assert_eq!(f("ID=nobara\nID_LIKE=\"rhel centos fedora\""), Family::Fedora);
        assert_eq!(f("ID=cachyos\nID_LIKE=arch"), Family::Arch);
        assert_eq!(f("ID=ubuntu\nID_LIKE=debian"), Family::Debian);
        assert_eq!(f("ID=linuxmint\nID_LIKE=\"ubuntu debian\""), Family::Debian);
        assert_eq!(f("ID=opensuse-tumbleweed\nID_LIKE=\"opensuse suse\""), Family::Suse);
        assert_eq!(f("ID=void"), Family::Other);
        assert_eq!(parse_os_release("ID=fedora\nPRETTY_NAME=\"Fedora Linux 45\"").1, "Fedora Linux 45");
    }

    #[test]
    fn the_fedora_helper_answer_is_understood() {
        let f = parse_fedora_json("noise\n{\"safe\": 357, \"held\": 26, \"total\": 383, \"held_by\": {\"boot and kernel\": 7, \"graphics drivers\": 8, \"the base of the system\": 9, \"our own session\": 1, \"major version 0 -> 1\": 1}}\n").expect("parsed");
        assert_eq!((f.total, f.safe), (383, Some(357)));
        assert_eq!(f.held[0], ("the base of the system".to_string(), 9), "the biggest group first");
        assert_eq!(f.held.iter().map(|h| h.1).sum::<usize>(), 26);
        assert!(parse_fedora_json("not json").is_none() && parse_fedora_json("{\"safe\": 1}").is_none());
    }

    #[test]
    fn other_package_managers_are_counted() {
        assert_eq!(count_lines(Family::Arch, "linux 6.1-1 -> 6.2-1\nfoo 1 -> 2\n\n"), 2);
        assert_eq!(count_lines(Family::Debian, "Reading...\nInst bash [1] (2 Debian)\nConf bash\nInst curl [1] (2)\n"), 2);
        assert_eq!(count_lines(Family::Suse, "S | Repository | Name | Current | Available\n--+--+--+--+--\nv | Main | zlib | 1 | 2\nv | Main | curl | 1 | 2\n"), 2);
        assert_eq!(count_lines(Family::Other, "anything"), 0);
    }

    #[test]
    fn every_family_with_a_check_has_an_update_command() {
        for f in [Family::Fedora, Family::Arch, Family::Debian, Family::Suse] {
            assert!(update_command(f).is_some(), "{f:?}");
        }
        assert!(update_command(Family::Other).is_none());
        assert!(update_command(Family::Fedora).unwrap().starts_with("nemo-fedora-update"));
    }
}
