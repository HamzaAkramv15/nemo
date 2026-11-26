// Реестр интеграций: программы и скрипты сообщают шеллу свои действия файлом-манифестом, а не кодом в шелле. Сегодня манифест даёт
// действия для поиска в лаунчере (то, что у Unity было «HUD», а у KRunner — «runner»); следующие поля (индикаторы, страницы настроек) встанут
// рядом в тот же файл. Нет демона и нет разбора на каждом кадре: манифесты читаются при открытии лаунчера.
//
// Манифест — JSON в `~/.config/nemo/integrations/*.json` (и в `$XDG_DATA_DIRS/nemo/integrations`); свои действия шелла и приложений Nemo
// лежат в config/integrations и вшиты в бинарь тем же форматом:
//   { "api": 1, "id": "my-tool", "name": "My Tool", "requires": "my-tool",
//     "actions": [ { "title": "Do it", "keywords": "word word", "icon": "gear", "command": ["my-tool", "--do-it"] } ] }
// `command` — список аргументов, без оболочки: ничто из набранного в поиске в него не попадает. `requires` — программа в PATH, без которой
// манифест молчит. Лишние поля игнорируются (манифест новее шелла не ломает его). Проверка: `nemo integrations check`.

use std::path::PathBuf;

/// версия формата манифеста
pub const API: u64 = 1;
const MAX_ACTIONS: usize = 64;
const MAX_TITLE: usize = 80;
const MAX_ARGV: usize = 32;
/// сколько .json читаем из одной папки: каталог на тысячи файлов — не манифесты
const MAX_FILES: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// id манифеста, чьё это действие
    pub from: String,
    pub title: String,
    pub keywords: String,
    pub icon: String,
    pub command: Vec<String>,
    /// свои действия Nemo: названия по-русски, переводятся при показе (чужие — как написал автор)
    pub builtin: bool,
}

#[derive(Default, Debug)]
pub struct Registry {
    pub actions: Vec<Action>,
    /// что не так с манифестами: «файл: причина»; пусто — всё хорошо
    pub problems: Vec<String>,
}

/// папки с манифестами в порядке приоритета (своя — первой)
pub fn dirs() -> Vec<PathBuf> {
    let mut v = vec![nemo_px::paths::config_dir().join("nemo/integrations")];
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    v.extend(data.split(':').filter(|d| !d.is_empty()).map(|d| PathBuf::from(d).join("nemo/integrations")));
    v
}

fn id_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 40 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// манифест → (id, действия); Err — причина, по которой он отвергнут целиком. Чистая функция: `requires` проверяет вызывающий
pub fn parse(text: &str, builtin: bool) -> Result<(String, Option<String>, Vec<Action>), String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let api = v.get("api").and_then(|x| x.as_u64()).ok_or("no \"api\" number")?;
    if api != API {
        return Err(format!("api {api}, this shell speaks {API}"));
    }
    let id = v.get("id").and_then(|x| x.as_str()).filter(|s| id_ok(s)).ok_or("\"id\" must be 1-40 characters of a-z 0-9 - _")?.to_string();
    let requires = v.get("requires").and_then(|x| x.as_str()).map(|s| s.to_string());
    let list = v.get("actions").and_then(|x| x.as_array()).ok_or("no \"actions\" list")?;
    if list.len() > MAX_ACTIONS {
        return Err(format!("{} actions, at most {MAX_ACTIONS}", list.len()));
    }
    let mut out = vec![];
    for (i, a) in list.iter().enumerate() {
        let title = a.get("title").and_then(|x| x.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).ok_or(format!("action {}: no title", i + 1))?;
        if title.chars().count() > MAX_TITLE {
            return Err(format!("action {}: title longer than {MAX_TITLE} characters", i + 1));
        }
        let command: Vec<String> = a
            .get("command")
            .and_then(|x| x.as_array())
            .map(|c| c.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
            .filter(|c: &Vec<String>| !c.is_empty() && c.len() <= MAX_ARGV && c[0].len() > 0 && !c[0].contains(['\n', '\0']))
            .ok_or(format!("action {}: \"command\" must be a list of 1-{MAX_ARGV} strings, the first one the program", i + 1))?;
        out.push(Action {
            from: id.clone(),
            title: title.to_string(),
            keywords: a.get("keywords").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            icon: a.get("icon").and_then(|x| x.as_str()).unwrap_or("gear").to_string(),
            command,
            builtin,
        });
    }
    Ok((id, requires, out))
}

const BUILTIN: &[(&str, &str)] = &[
    ("nemo-shell.json", include_str!("../../config/integrations/nemo-shell.json")),
    ("nemo-shot.json", include_str!("../../config/integrations/nemo-shot.json")),
    ("nemo-files.json", include_str!("../../config/integrations/nemo-files.json")),
    ("nemo-view.json", include_str!("../../config/integrations/nemo-view.json")),
    ("nemo-play.json", include_str!("../../config/integrations/nemo-play.json")),
];

/// все манифесты: вшитые, потом из папок; повтор id пропускается (первый побеждает, свой — раньше системного); `have` — есть ли программа
pub fn load_with(dirs: &[PathBuf], have: &dyn Fn(&str) -> bool) -> Registry {
    let mut reg = Registry::default();
    let mut seen: Vec<String> = vec![];
    let mut take = |name: &str, text: &str, builtin: bool, reg: &mut Registry| match parse(text, builtin) {
        Ok((id, requires, acts)) => {
            if seen.contains(&id) {
                return;
            }
            seen.push(id);
            if requires.as_deref().is_none_or(have) {
                reg.actions.extend(acts);
            }
        }
        Err(e) => reg.problems.push(format!("{name}: {e}")),
    };
    // своё сначала: человек может заменить вшитое манифестом с тем же id
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
        files.sort();
        for p in files.into_iter().take(MAX_FILES) {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            // манифест — не больше 64 КБ: больше — это не он
            match std::fs::metadata(&p) {
                Ok(m) if m.len() <= 65536 => {}
                _ => {
                    reg.problems.push(format!("{name}: larger than 64 KB"));
                    continue;
                }
            }
            if let Ok(text) = std::fs::read_to_string(&p) {
                take(&name, &text, false, &mut reg);
            }
        }
    }
    for (name, text) in BUILTIN {
        take(name, text, true, &mut reg);
    }
    reg
}

pub fn load() -> Registry {
    load_with(&dirs(), &|p| crate::proc::have(p))
}

/// `nemo integrations check`: что нашлось и что не так
pub fn check_cli() -> i32 {
    let reg = load();
    let mine = reg.actions.iter().filter(|a| !a.builtin).count();
    println!("{} actions ({} of them from your manifests)", reg.actions.len(), mine);
    for a in reg.actions.iter().filter(|a| !a.builtin) {
        println!("  {}: {}  →  {}", a.from, a.title, a.command.join(" "));
    }
    for p in &reg.problems {
        println!("problem: {p}");
    }
    i32::from(!reg.problems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{ "api": 1, "id": "my-tool", "name": "My Tool", "actions": [ { "title": "Do it", "keywords": "go run", "command": ["my-tool", "--go"] } ], "future": 5 }"#;

    #[test]
    fn a_valid_manifest_parses_and_unknown_fields_are_ignored() {
        let (id, req, acts) = parse(OK, false).unwrap();
        assert_eq!(id, "my-tool");
        assert_eq!(req, None);
        assert_eq!(acts.len(), 1);
        assert_eq!(acts[0].command, vec!["my-tool", "--go"]);
        assert_eq!(acts[0].icon, "gear", "a default icon");
        assert!(!acts[0].builtin);
    }

    #[test]
    fn broken_manifests_say_why() {
        for (text, why) in [
            ("nope", "not JSON"),
            (r#"{"id":"a","actions":[]}"#, "no \"api\""),
            (r#"{"api":2,"id":"a","actions":[]}"#, "api 2"),
            (r#"{"api":1,"id":"Bad Id","actions":[]}"#, "\"id\""),
            (r#"{"api":1,"id":"a"}"#, "no \"actions\""),
            (r#"{"api":1,"id":"a","actions":[{"command":["x"]}]}"#, "no title"),
            (r#"{"api":1,"id":"a","actions":[{"title":"t"}]}"#, "\"command\""),
            (r#"{"api":1,"id":"a","actions":[{"title":"t","command":[]}]}"#, "\"command\""),
            (r#"{"api":1,"id":"a","actions":[{"title":"t","command":[""]}]}"#, "\"command\""),
        ] {
            let e = parse(text, false).unwrap_err();
            assert!(e.contains(why), "{text} -> {e}");
        }
        let long = format!(r#"{{"api":1,"id":"a","actions":[{{"title":"{}","command":["x"]}}]}}"#, "x".repeat(81));
        assert!(parse(&long, false).unwrap_err().contains("title longer"));
        let many = format!(r#"{{"api":1,"id":"a","actions":[{}]}}"#, vec![r#"{"title":"t","command":["x"]}"#; 65].join(","));
        assert!(parse(&many, false).unwrap_err().contains("at most"));
    }

    #[test]
    fn the_builtin_manifests_are_valid_and_their_programs_are_required() {
        for (name, text) in BUILTIN {
            let (_, _, acts) = parse(text, true).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!acts.is_empty(), "{name}");
        }
        let none = load_with(&[], &|_| false);
        assert!(none.problems.is_empty());
        assert!(none.actions.iter().all(|a| a.from == "nemo-shell"), "without the apps only the shell's own actions remain");
        let all = load_with(&[], &|_| true);
        assert!(all.actions.len() > none.actions.len());
        assert!(all.actions.iter().any(|a| a.from == "nemo-shot" && a.command.last().map(|s| s.as_str()) == Some("record")));
    }

    #[test]
    fn your_own_folder_loads_and_wins_over_a_builtin_with_the_same_id() {
        let d = std::env::temp_dir().join(format!("nemo-integ-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("mine.json"), OK).unwrap();
        std::fs::write(d.join("broken.json"), "{").unwrap();
        std::fs::write(d.join("override.json"), r#"{"api":1,"id":"nemo-view","name":"x","actions":[{"title":"my viewer","command":["imv"]}]}"#).unwrap();
        std::fs::write(d.join("notes.txt"), "ignored").unwrap();
        let r = load_with(&[d.clone()], &|_| true);
        assert!(r.actions.iter().any(|a| a.title == "Do it" && !a.builtin));
        assert!(r.problems.iter().any(|p| p.starts_with("broken.json")), "{:?}", r.problems);
        assert!(r.actions.iter().any(|a| a.title == "my viewer"));
        assert!(!r.actions.iter().any(|a| a.from == "nemo-view" && a.builtin), "the user's manifest replaced the built-in one");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_oversized_manifest_is_refused_not_read() {
        let d = std::env::temp_dir().join(format!("nemo-integ-big-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("big.json"), vec![b' '; 70000]).unwrap();
        let r = load_with(&[d.clone()], &|_| true);
        assert!(r.problems.iter().any(|p| p.contains("larger than 64 KB")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
