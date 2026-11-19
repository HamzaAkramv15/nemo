// Что известно об обновлении кроме его заметок: сообщения о проблемах, заметки безопасности и более новый выпуск
// в репозитории (GitHub). Ничего не объявляется «безопасным»: результат — только то, что нашлось, откуда и когда.
// Нашлось ноль — это «сообщений не видно», а не гарантия; не достучались — так и сказано.

use serde::{Deserialize, Serialize};

/// заметки выпуска по смыслу: сколько исправлений, улучшений, есть ли про безопасность и перезапуск
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Kinds {
    pub fixes: usize,
    pub other: usize,
    pub security: bool,
    pub niri: bool,
}

/// строки заметок → виды: по словам в самих строках (то, что написал автор выпуска), без догадок сверх этого
pub fn classify(notes: &[String]) -> Kinds {
    let mut k = Kinds::default();
    for n in notes {
        let l = n.to_lowercase();
        let has = |w: &[&str]| w.iter().any(|x| l.contains(x));
        if has(&["security", "vulnerab", "cve-", "безопасн", "уязвим"]) { // i18n-ok
            k.security = true;
        }
        if has(&["niri", "login screen", "greet", "экран входа"]) {
            k.niri = true;
        }
        if has(&["fix", "bug", "crash", "исправ", "ошибк", "сбой", "вылет"]) { // i18n-ok
            k.fixes += 1;
        } else {
            k.other += 1;
        }
    }
    k
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Report {
    /// для какой версии и когда проверено (unix)
    pub version: String,
    pub at: u64,
    /// открытые сообщения о проблемах: (номер, заголовок)
    pub open: Vec<(u64, String)>,
    /// закрытые с выхода этого выпуска
    pub closed: usize,
    /// опубликованные заметки безопасности (заголовки)
    pub advisories: Vec<String>,
    /// выпуск новее проверяемого
    pub newer: String,
    /// выпуск помечен как предварительный
    pub prerelease: bool,
    /// что не удалось спросить
    pub errors: Vec<String>,
    /// сколько источников ответило
    pub answered: usize,
}

/// «https://github.com/О/Р.git» → (О, Р)
pub fn repo_of(url: &str) -> Option<(String, String)> {
    let r = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let r = r.strip_prefix("https://github.com/").or_else(|| r.strip_prefix("git@github.com:"))?;
    let (o, n) = r.split_once('/')?;
    (!o.is_empty() && !n.is_empty() && !n.contains('/')).then(|| (o.to_string(), n.to_string()))
}

/// «3.2.0» → (3, 2, 0); любая приставка v ничего не меняет
pub fn parse_ver(v: &str) -> Option<(u32, u32, u32)> {
    let mut p = v.trim().trim_start_matches('v').split('.');
    Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?))
}

/// ответ /issues → открытые (по меткам bug/regression или версии в заголовке) и сколько закрыто; запросы на слияние не считаются
pub fn read_issues(json: &str, version: &str) -> Option<(Vec<(u64, String)>, usize)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut open = vec![];
    let mut closed = 0;
    for i in v.as_array()? {
        if i.get("pull_request").is_some() {
            continue;
        }
        let title = i["title"].as_str().unwrap_or("").to_string();
        let labelled = i["labels"].as_array().is_some_and(|l| l.iter().any(|x| matches!(x["name"].as_str().map(str::to_lowercase).as_deref(), Some("bug" | "regression" | "crash"))));
        if !(labelled || (!version.is_empty() && title.contains(version))) {
            continue;
        }
        if i["state"] == "open" {
            open.push((i["number"].as_u64().unwrap_or(0), title));
        } else {
            closed += 1;
        }
    }
    Some((open, closed))
}

/// ответ /releases → (есть ли выпуск новее проверяемого, предварительный ли он сам)
pub fn read_releases(json: &str, version: &str) -> Option<(String, bool)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let cur = parse_ver(version)?;
    let mut newer: Option<((u32, u32, u32), String)> = None;
    let mut pre = false;
    for r in v.as_array()? {
        if r["draft"] == true {
            continue;
        }
        let tag = r["tag_name"].as_str().unwrap_or("");
        let Some(pv) = parse_ver(tag) else { continue };
        if pv == cur {
            pre = r["prerelease"] == true;
        } else if pv > cur && r["prerelease"] != true && newer.as_ref().is_none_or(|(b, _)| pv > *b) {
            newer = Some((pv, tag.trim_start_matches('v').to_string()));
        }
    }
    Some((newer.map(|n| n.1).unwrap_or_default(), pre))
}

/// ответ /security-advisories → заголовки опубликованных (черновики и закрытые не в счёт)
pub fn read_advisories(json: &str) -> Option<Vec<String>> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    Some(v.as_array()?.iter().filter(|a| a["state"] == "published").filter_map(|a| a["summary"].as_str().map(String::from)).collect())
}

fn get(url: &str) -> Result<String, String> {
    let o = std::process::Command::new("curl")
        .args(["-sS", "-m", "12", "--fail", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: nemo-shell", url])
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().lines().last().unwrap_or("").to_string())
    }
}

/// спросить GitHub про `version` репозитория `repo` (URL); ничего не меняет, сеть — только чтение
pub fn check(repo: &str, version: &str, since: &str) -> Report {
    let mut r = Report { version: version.to_string(), at: crate::update::now(), ..Default::default() };
    let Some((o, n)) = repo_of(repo) else {
        r.errors.push(crate::tf!("репозиторий не на GitHub: проверять негде", "the repository is not on GitHub: nowhere to look", "o repositório não está no GitHub: sem onde verificar"));
        return r;
    };
    let base = format!("https://api.github.com/repos/{o}/{n}");
    let since_q = if since.is_empty() { String::new() } else { format!("&since={since}") };
    match get(&format!("{base}/issues?state=all&per_page=100{since_q}")).map(|t| read_issues(&t, version)) {
        Ok(Some((open, closed))) => {
            (r.open, r.closed) = (open, closed);
            r.answered += 1;
        }
        Ok(None) => r.errors.push(crate::tf!("сообщения о проблемах: ответ не разобран", "problem reports: the answer could not be read", "relatos de problemas: resposta ilegível")),
        Err(e) => r.errors.push(format!("{}: {e}", crate::tf!("сообщения о проблемах", "problem reports", "relatos de problemas"))),
    }
    match get(&format!("{base}/releases?per_page=20")).map(|t| read_releases(&t, version)) {
        Ok(Some((newer, pre))) => {
            (r.newer, r.prerelease) = (newer, pre);
            r.answered += 1;
        }
        Ok(None) => r.errors.push(crate::tf!("выпуски: ответ не разобран", "releases: the answer could not be read", "versões: resposta ilegível")),
        Err(e) => r.errors.push(format!("{}: {e}", crate::tf!("выпуски", "releases", "versões"))),
    }
    // без прав владельца GitHub отдаёт только опубликованные заметки; 404 — их нет вовсе, и это не ошибка
    match get(&format!("{base}/security-advisories?per_page=30")) {
        Ok(t) => {
            if let Some(a) = read_advisories(&t) {
                r.advisories = a;
                r.answered += 1;
            }
        }
        Err(e) if e.contains("404") => r.answered += 1,
        Err(e) => r.errors.push(format!("{}: {e}", crate::tf!("заметки безопасности", "security advisories", "avisos de segurança"))),
    }
    r
}

pub fn path() -> std::path::PathBuf {
    nemo_px::paths::state_dir().join("update-reports.json")
}

pub fn load() -> Option<Report> {
    serde_json::from_str(&std::fs::read_to_string(path()).ok()?).ok()
}

pub fn save(r: &Report) {
    if let Ok(t) = serde_json::to_string(r) {
        let _ = crate::config::write_atomic(&path(), &t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn notes_are_sorted_by_what_their_own_words_say() {
        let k = classify(&s(&["Fix: crash when the lid closes", "files: new sorting", "Security: the password is wiped from memory sooner", "login screen: caps lock badge", "исправлена ошибка в календаре"])); // i18n-ok
        assert_eq!((k.fixes, k.other, k.security, k.niri), (2, 3, true, true));
        assert_eq!(classify(&[]), Kinds::default());
        assert!(!classify(&s(&["calendar: week numbers"])).security);
    }

    #[test]
    fn a_github_address_gives_owner_and_name() {
        assert_eq!(repo_of("https://github.com/HamzaAkramv15/nemo.git"), Some(("HamzaAkramv15".into(), "nemo".into())));
        assert_eq!(repo_of("git@github.com:a/b"), Some(("a".into(), "b".into())));
        assert_eq!(repo_of("https://example.org/a/b"), None);
        assert_eq!(repo_of("https://github.com/a"), None);
    }

    #[test]
    fn issues_count_bugs_and_this_version_but_not_pull_requests() {
        let j = r#"[
          {"number":1,"title":"Crash on resume","state":"open","labels":[{"name":"bug"}]},
          {"number":2,"title":"3.2.0: bar is blank","state":"open","labels":[]},
          {"number":3,"title":"Old regression","state":"closed","labels":[{"name":"Regression"}]},
          {"number":4,"title":"Nice idea","state":"open","labels":[{"name":"idea"}]},
          {"number":5,"title":"fix bug","state":"open","labels":[{"name":"bug"}],"pull_request":{}}
        ]"#;
        let (open, closed) = read_issues(j, "3.2.0").unwrap();
        assert_eq!(open, vec![(1, "Crash on resume".to_string()), (2, "3.2.0: bar is blank".to_string())]);
        assert_eq!(closed, 1);
        assert!(read_issues("not json", "1.0.0").is_none());
    }

    #[test]
    fn a_newer_stable_release_is_reported_and_prereleases_are_not_offered() {
        let j = r#"[
          {"tag_name":"v3.3.0","prerelease":true,"draft":false},
          {"tag_name":"v3.2.1","prerelease":false,"draft":false},
          {"tag_name":"v3.2.0","prerelease":false,"draft":false},
          {"tag_name":"v9.9.9","prerelease":false,"draft":true}
        ]"#;
        assert_eq!(read_releases(j, "3.2.0"), Some(("3.2.1".to_string(), false)));
        assert_eq!(read_releases(j, "3.2.1"), Some((String::new(), false)));
        assert_eq!(read_releases("[]", "garbage"), None);
    }

    #[test]
    fn only_published_advisories_count() {
        let j = r#"[{"state":"published","summary":"Path traversal in import"},{"state":"draft","summary":"secret"}]"#;
        assert_eq!(read_advisories(j), Some(vec!["Path traversal in import".to_string()]));
        assert_eq!(read_advisories("[]"), Some(vec![]));
    }
}
