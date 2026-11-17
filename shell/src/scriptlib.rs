// Готовые скрипты для бара и виджетов: файлы лежат в config/scripts и вшиты в бинарь (сеть не
// нужна, чужой код не скачивается). `yami scripts add NAME` кладёт копию в ~/.config/yami/scripts
// исполняемым файлом; дальше это обычный твой скрипт (plugins.rs): модуль `x:NAME` в настройках → бар
// и источник виджета «скрипт». Свой файл с тем же именем не трогается.
// i18n: inline pairs

use std::os::unix::fs::PermissionsExt;

/// (имя файла, текст)
pub const ALL: &[(&str, &str)] = &[
    ("uptime", include_str!("../../config/scripts/uptime")),
    ("load", include_str!("../../config/scripts/load")),
    ("disk", include_str!("../../config/scripts/disk")),
    ("memtop", include_str!("../../config/scripts/memtop")),
    ("cpufreq", include_str!("../../config/scripts/cpufreq")),
    ("power", include_str!("../../config/scripts/power")),
    ("kernel", include_str!("../../config/scripts/kernel")),
    ("vpn", include_str!("../../config/scripts/vpn")),
    ("docker", include_str!("../../config/scripts/docker")),
    ("flatpak", include_str!("../../config/scripts/flatpak")),
    ("swap", include_str!("../../config/scripts/swap")),
    ("cputemp", include_str!("../../config/scripts/cputemp")),
    ("ipaddr", include_str!("../../config/scripts/ipaddr")),
    ("wifi", include_str!("../../config/scripts/wifi")),
    ("yearprogress", include_str!("../../config/scripts/yearprogress")),
    ("utc", include_str!("../../config/scripts/utc")),
    ("processes", include_str!("../../config/scripts/processes")),
    ("fan", include_str!("../../config/scripts/fan")),
    ("gitdir", include_str!("../../config/scripts/gitdir")),
    ("moon", include_str!("../../config/scripts/moon")),
];

/// строка «# yami-script: …» — что скрипт показывает
pub fn about(body: &str) -> &str {
    body.lines().find_map(|l| l.strip_prefix("# yami-script:")).map(str::trim).unwrap_or("")
}

/// положить копию; false — такой файл уже есть (он твой) или записать не вышло
pub fn add(name: &str) -> Result<(), String> {
    let Some((_, body)) = ALL.iter().find(|(n, _)| *n == name) else {
        return Err(crate::tf!("нет такого скрипта: {}", "no such script: {}", "não há esse script: {}", name));
    };
    let dir = crate::plugins::dir();
    let p = dir.join(name);
    if p.exists() {
        return Err(crate::tf!("{} уже есть, он твой: не трогаю", "{} is already there, it is yours: left alone", "{} já existe, é seu: não mexo", name));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    crate::config::write_atomic(&p, body).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

/// `yami scripts [list | add NAME… | add all]`
pub fn cli(y: &crate::cmd::Sty, args: &[&str]) -> i32 {
    match args {
        [] | ["list"] => {
            y.section(&crate::tf!("готовые скрипты (бар и виджеты)", "ready scripts (bar and widgets)", "scripts prontos (barra e widgets)"));
            let have = crate::plugins::scripts();
            for (n, body) in ALL {
                let mark = if have.iter().any(|h| h == n) { y.ok("✓") } else { y.dim("·") };
                println!(" {mark} {n:<13} {}", y.dim(crate::i18n::tr(about(body))));
            }
            println!("   {}", y.dim(&crate::tf!("yami scripts add ИМЯ | all; потом: настройки → бар (x:ИМЯ) или виджеты → скрипт", "yami scripts add NAME | all; then settings → bar (x:NAME) or widgets → script", "yami scripts add NOME | all; depois: configurações → barra (x:NOME) ou widgets → script")));
            0
        }
        ["add", names @ ..] if !names.is_empty() => {
            let all: Vec<&str> = if names == ["all"] { ALL.iter().map(|(n, _)| *n).collect() } else { names.to_vec() };
            let mut rc = 0;
            for n in all {
                match add(n) {
                    Ok(()) => println!(" {} {n}", y.ok("✓")),
                    Err(e) => {
                        println!(" {} {e}", y.warn("!"));
                        rc = 1;
                    }
                }
            }
            rc
        }
        _ => {
            eprintln!(" {} yami scripts [list | add {} | add all]", y.bad("✕"), crate::tf!("ИМЯ", "NAME", "NOME"));
            2
        }
    }
}

#[cfg(test)]
mod tests {
    /// каждый скрипт — исполняемый sh с описанием, а имя годится для модуля бара
    #[test]
    fn every_script_is_complete() {
        for (name, body) in super::ALL {
            assert!(crate::plugins::name_ok(name), "{name}");
            assert!(body.starts_with("#!/bin/sh\n"), "{name}: shebang");
            assert!(!super::about(body).is_empty(), "{name}: no «# yami-script:» line");
            assert!(body.contains("\"text\""), "{name}: prints no JSON text");
            assert!(body.contains("\"interval\""), "{name}: no interval");
        }
        let mut names: Vec<&str> = super::ALL.iter().map(|(n, _)| *n).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), super::ALL.len(), "duplicate names");
    }
}
