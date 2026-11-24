// Железо за небольшим интерфейсом: «провайдер» сам определяет, есть ли у этой машины его регулятор, и отдаёт список элементов.
// Интерфейс показывает только то, что нашлось (питание в настройках → удобство), и ничего не требует от производителя.
// Первый провайдер — предел заряда батареи (sysfs charge_control_end_threshold); asusd, fwupd и другие встают рядом такими же.
// Запись в sysfs требует прав: через pkexec (окно пароля рисует наш агент polkit).

use std::path::{Path, PathBuf};

/// один регулятор: чей он, как назван, текущее значение и допустимые
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    pub provider: &'static str,
    pub id: &'static str,
    pub label: &'static str,
    pub value: String,
    pub choices: &'static [&'static str],
}

pub const CHARGE_CHOICES: &[&str] = &["60", "70", "80", "90", "100"];

/// файлы предела заряда у всех батарей в `root` (/sys/class/power_supply)
pub fn charge_files(root: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(root) else { return vec![] };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("BAT"))
        .map(|e| e.path().join("charge_control_end_threshold"))
        .filter(|p| p.is_file())
        .collect();
    v.sort();
    v
}

/// ближайшее допустимое значение к тому, что отдала батарея (у разных ноутбуков бывает 55 или 95)
pub fn nearest(v: u32) -> &'static str {
    CHARGE_CHOICES.iter().min_by_key(|c| c.parse::<i64>().map_or(i64::MAX, |c| (c - v as i64).abs())).copied().unwrap_or("100")
}

/// регуляторы, которые есть у этой машины
pub fn controls_in(root: &Path) -> Vec<Control> {
    let mut out = vec![];
    if let Some(v) = charge_files(root).first().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| t.trim().parse::<u32>().ok()) {
        out.push(Control { provider: "battery", id: "charge_limit", label: "предел заряда", value: nearest(v).to_string(), choices: CHARGE_CHOICES });
    }
    out
}

pub fn controls() -> Vec<Control> {
    // картинки для сравнения (кадр времени заморожен) не зависят от батареи этой машины: образец с пределом 80 %
    if std::env::var_os("NEMO_FROZEN_NOW").is_some() {
        return vec![Control { provider: "battery", id: "charge_limit", label: "предел заряда", value: "80".into(), choices: CHARGE_CHOICES }];
    }
    controls_in(Path::new("/sys/class/power_supply"))
}

/// следующее значение регулятора по кругу
pub fn next_choice(c: &Control) -> &'static str {
    let i = c.choices.iter().position(|x| *x == c.value).map_or(0, |i| (i + 1) % c.choices.len());
    c.choices[i]
}

/// записать предел во все батареи через pkexec tee; Err — что показать пользователю. Значение — только из CHARGE_CHOICES, путь — из списка выше:
/// ничего введённого руками в команду не попадает
pub fn set_charge_limit(root: &Path, value: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    if !CHARGE_CHOICES.contains(&value) {
        return Err(format!("bad value {value}"));
    }
    let files = charge_files(root);
    if files.is_empty() {
        return Err("no battery limit".into());
    }
    for f in files {
        let mut child = Command::new("pkexec").arg("tee").arg(&f).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
        if let Some(mut si) = child.stdin.take() {
            let _ = writeln!(si, "{value}");
        }
        let st = child.wait().map_err(|e| e.to_string())?;
        if !st.success() {
            return Err(format!("{} ({})", crate::i18n::tr("не удалось записать предел заряда"), st));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(limit: &str) -> tempdir::Dir {
        let d = tempdir::Dir::new();
        let b = d.path().join("BAT0");
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(b.join("charge_control_end_threshold"), format!("{limit}\n")).unwrap();
        std::fs::create_dir_all(d.path().join("AC")).unwrap();
        d
    }

    /// каталог на время теста (в tests не тянем крейт ради двух вызовов)
    mod tempdir {
        pub struct Dir(std::path::PathBuf);
        impl Dir {
            pub fn new() -> Dir {
                let p = std::env::temp_dir().join(format!("nemo-hw-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
                std::fs::create_dir_all(&p).unwrap();
                Dir(p)
            }
            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn the_control_shows_only_where_the_machine_has_it() {
        let none = tempdir::Dir::new();
        assert!(controls_in(none.path()).is_empty(), "no battery: no control");
        let d = fake("80");
        let c = controls_in(d.path());
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].value, "80");
        assert_eq!(next_choice(&c[0]), "90");
    }

    #[test]
    fn odd_limits_snap_to_the_nearest_choice_and_the_cycle_wraps() {
        assert_eq!(nearest(55), "60");
        assert_eq!(nearest(95), "90");
        let c = Control { provider: "battery", id: "charge_limit", label: "x", value: "100".into(), choices: CHARGE_CHOICES };
        assert_eq!(next_choice(&c), "60");
    }

    #[test]
    fn only_listed_values_reach_the_command() {
        let d = fake("80");
        assert!(set_charge_limit(d.path(), "80; rm -rf /").is_err());
        assert!(set_charge_limit(d.path(), "42").is_err());
        assert!(set_charge_limit(tempdir::Dir::new().path(), "80").is_err(), "no files: nothing to write");
    }
}
