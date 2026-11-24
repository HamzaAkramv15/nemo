// Модули бара в одной таблице: идентификатор, название в настройках, группа (между группами бар ставит разделитель). Её читают вкладка
// «бар» (названия, список «добавить»), нормализация раскладки (config.rs) и проверки; рисуют модули две функции в bar.rs (горизонтальный и
// боковой бар): тест ниже следит, чтобы у каждого модуля из таблицы было и то и другое, а таблица не разъезжалась с кодом.
// Новый модуль — строка здесь и ветка в обеих функциях bar.rs.

pub struct Module {
    pub id: &'static str,
    /// название — русская строка, переводится при показе
    pub name: &'static str,
    pub group: i32,
}

pub const MODULES: &[Module] = &[
    Module { id: "launcher", name: "меню программ", group: 0 },
    Module { id: "workspaces", name: "столы", group: 1 },
    Module { id: "mode", name: "стол / лента", group: 2 },
    Module { id: "tasks", name: "окна", group: 7 },
    Module { id: "media", name: "музыка", group: 5 },
    Module { id: "tray", name: "трей", group: 3 },
    Module { id: "net", name: "сеть", group: 4 },
    Module { id: "netspeed", name: "скорость сети", group: 4 },
    Module { id: "bt", name: "bluetooth", group: 4 },
    Module { id: "kb", name: "раскладка", group: 4 },
    Module { id: "cpu", name: "процессор", group: 4 },
    Module { id: "ram", name: "память", group: 4 },
    Module { id: "temp", name: "температура", group: 4 },
    Module { id: "gpu", name: "видеокарта", group: 7 },
    Module { id: "disk", name: "диск", group: 7 },
    Module { id: "weather", name: "погода", group: 7 },
    Module { id: "vol", name: "звук", group: 5 },
    Module { id: "bat", name: "батарея", group: 4 },
    Module { id: "profile", name: "профиль питания", group: 4 },
    Module { id: "night", name: "ночной свет", group: 4 },
    Module { id: "caffeine", name: "кофе", group: 4 },
    Module { id: "privacy", name: "приватность", group: 4 },
    Module { id: "clock", name: "часы", group: 6 },
    Module { id: "notif", name: "уведомления", group: 6 },
    Module { id: "settings", name: "настройки", group: 6 },
    Module { id: "power", name: "питание", group: 6 },
    Module { id: "sep", name: "разделитель", group: 7 },
];

/// название модуля для настроек; неизвестный (скрипт «x:имя» и т. п.) — как есть
pub fn name(id: &str) -> &str {
    MODULES.iter().find(|m| m.id == id).map_or(id, |m| m.name)
}

/// группа модуля: соседние модули разных групп разделяет разделитель; скриптам и неизвестным — последняя
pub fn group(id: &str) -> i32 {
    MODULES.iter().find(|m| m.id == id).map_or(7, |m| m.group)
}

pub fn ids() -> impl Iterator<Item = &'static str> {
    MODULES.iter().map(|m| m.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// модуль рисуется веткой `"id" =>` в обеих функциях bar.rs (горизонтальной и боковой); разделитель — не модуль, а рамка между ними
    #[test]
    fn every_module_has_a_branch_in_both_bars() {
        let src = include_str!("bar.rs");
        // «tasks» растягивается на свободное место и рисуется отдельной ветвью (tasks_width), не обычным модулем
        for m in MODULES.iter().filter(|m| !matches!(m.id, "sep" | "tasks")) {
            let n = src.matches(&format!("\"{}\" =>", m.id)).count() + src.matches(&format!("\"{}\" |", m.id)).count();
            assert!(n >= 2, "module «{}» is drawn {} time(s) in bar.rs, the horizontal and the side bar both need it", m.id, n);
        }
    }

    #[test]
    fn ids_are_unique_and_names_filled() {
        let mut seen = std::collections::HashSet::new();
        for m in MODULES {
            assert!(seen.insert(m.id), "duplicate module id {}", m.id);
            assert!(!m.name.is_empty());
        }
    }

    #[test]
    fn the_defaults_only_name_known_modules() {
        let d = crate::config::Settings::default();
        for id in d.bar_left.iter().chain(&d.bar_center).chain(&d.bar_right) {
            assert!(MODULES.iter().any(|m| m.id == id) || id.starts_with("x:"), "default bar has unknown module {id}");
        }
    }
}
