// Запись экрана, которую начал `nemo-shot record` (его wf-recorder): шелл только смотрит, идёт ли она, показывает красную точку в баре
// (privacy.rs) и останавливает запись при блокировке. Состояние — файл $XDG_RUNTIME_DIR/nemo-record.pid «pid путь»; проверка — раз в тик.

use std::path::PathBuf;

fn state_file() -> PathBuf {
    PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("nemo-record.pid")
}

/// файл идущей записи, если она идёт (процесс жив)
pub fn running() -> Option<String> {
    let t = std::fs::read_to_string(state_file()).ok()?;
    let (pid, file) = t.trim().split_once(' ')?;
    let pid: i32 = pid.parse().ok()?;
    // SAFETY: сигнал 0 только проверяет, что процесс есть
    (unsafe { libc::kill(pid, 0) } == 0).then(|| file.to_string())
}

/// остановить запись тем же вызовом, что и кнопка в оверлее (он дописывает файл и говорит «сохранена»)
pub fn stop() {
    if running().is_some() {
        crate::app::App::spawn(&["nemo-shot".to_string(), "record".to_string(), "stop".to_string()]);
    }
}
