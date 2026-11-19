// Где nemo держит своё в домашней папке: всё в ~/.config/nemo, как у других шеллов.
//   shell.json                     настройки шелла (до 3.0 — ~/.config/nemo-shell.json)
//   style.json, style.<модуль>.json   вид модулей; files.json, shot.json, agents.json — их настройки
//   scripts/                       модули бара и виджеты из скриптов (до 3.0 — modules/)
//   themes/, templates/            свои темы и шаблоны перекраски
//   niri/                          часть nemo в конфиге niri (niricfg.rs)
// `migrate` переносит файлы старых версий; зовётся при каждом запуске, без старых файлов — пара stat.

use std::path::{Path, PathBuf};
pub use nemo_px::paths::nemo_dir;

pub fn niri_dir() -> PathBuf {
    nemo_dir().join("niri")
}

pub fn scripts_dir() -> PathBuf {
    nemo_dir().join("scripts")
}

/// файлы niri до 3.0 (рядом с config.kdl) → их имена в ~/.config/nemo/niri
// файлы niri, которые писала версия до 3.0 под прежним именем (yami): что куда переезжает
pub const OLD_NIRI: [(&str, &str); 5] = [("yami-binds.kdl", "binds.kdl"), ("yami-fx.kdl", "rules.kdl"), ("yami-colors.kdl", "colors.kdl"), ("yami-cursor.kdl", "cursor.kdl"), ("yami-outputs.kdl", "outputs.kdl")];

fn mv(from: &Path, to: &Path) {
    if from.exists() && !to.exists() {
        if let Some(d) = to.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::rename(from, to);
    }
}

pub fn migrate() {
    let cfg = nemo_px::paths::config_dir();
    let y = nemo_dir();
    mv(&cfg.join("yami-shell.json"), &y.join("shell.json"));
    mv(&y.join("modules"), &scripts_dir());
    // сборка темы SDDM: экран входа — nemo-greet с 2.6 (nemo-cursor не трогать: это рабочая папка
    // установщика, installer/login.sh)
    let _ = std::fs::remove_dir_all(nemo_px::paths::cache_dir().join("nemo/sddm-nemo"));
    let _ = std::fs::remove_dir_all(nemo_px::paths::cache_dir().join("nemo/sddm-yami"));
    crate::niricfg::migrate_renamed(&cfg.join("niri"));
    let n = cfg.join("niri");
    if !OLD_NIRI.iter().any(|(o, _)| n.join(o).exists()) && !n.join("yami-local.kdl").exists() && !n.join(".yami").exists() {
        return;
    }
    // сначала новая часть, потом правка config.kdl: niri перечитывает его сразу после записи
    let set = crate::config::Settings::load();
    for (old, new) in OLD_NIRI {
        mv(&n.join(old), &niri_dir().join(new));
        // новый уже был: старый не нужен, шелл перепишет новый сам
        let _ = std::fs::remove_file(n.join(old));
    }
    crate::niricfg::write_nemo(&set);
    crate::niricfg::migrate_old_config(&n);
    let _ = std::fs::remove_file(n.join("yami-local.kdl"));
    let _ = std::fs::remove_file(n.join(".yami"));
}

/// откат на версию до 3.0: она ищет настройки и скрипты на старых местах. Копия, не перенос:
/// возвращение на новую версию найдёт всё на месте
pub fn back_to_old() {
    // версии до переименования искали всё в ~/.config/yami: копия настроек туда
    let cfg = nemo_px::paths::config_dir();
    let (y, old) = (nemo_dir(), cfg.join("yami"));
    let _ = std::fs::create_dir_all(&old);
    for f in ["shell.json", "files.json", "style.json", "agents.json", "shot.json"] {
        if y.join(f).exists() && !old.join(f).exists() {
            let _ = std::fs::copy(y.join(f), old.join(f));
        }
    }
    if scripts_dir().is_dir() && !old.join("scripts").exists() {
        let _ = std::os::unix::fs::symlink(scripts_dir(), old.join("scripts"));
    }
}
