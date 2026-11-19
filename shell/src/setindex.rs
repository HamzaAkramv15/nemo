// Поиск по настройкам: каждая настройка — запись (вкладка, подпись, слова-синонимы). Запрос из поля поиска
// ищет по подписи (как она написана и как переведена на язык интерфейса), по синонимам и по названию вкладки;
// все слова запроса должны найтись. Результаты — страница вместо вкладки: щелчок открывает вкладку.
// Записи (setindex_data.rs) сделаны из подписей вкладок; тест the_index_covers_every_setting_label не даёт
// добавить настройку и забыть её здесь.

use crate::app::State;
use crate::font::PX;
use crate::gfx::R;
use crate::settings::tabs;
use crate::ui::{Act, Cur, Tone, Ui};

pub struct Entry {
    pub tab: &'static str,
    /// подпись, как в коде (по-русски, ключ перевода)
    pub label: &'static str,
    /// дополнительные слова: как ещё называют эту настройку
    pub kw: &'static str,
}

include!("setindex_data.rs");

/// слова запроса: по нижнему регистру, без пустых
fn words(q: &str) -> Vec<String> {
    q.to_lowercase().split_whitespace().map(str::to_string).collect()
}

/// подходит ли запись: каждое слово есть в подписи (русской или переведённой), синонимах или названии вкладки
pub fn matches(e: &Entry, q: &str) -> bool {
    let w = words(q);
    if w.is_empty() {
        return false;
    }
    let tab = crate::settings::find(e.tab).map(|t| format!("{} {}", t.name, crate::i18n::tr(t.name))).unwrap_or_default();
    let hay = format!("{} {} {} {}", e.label, crate::i18n::tr(e.label), e.kw, tab).to_lowercase();
    w.iter().all(|x| hay.contains(x.as_str()))
}

/// найденные настройки: сначала те, где слово в самой подписи, потом по синонимам
pub fn search(q: &str) -> Vec<&'static Entry> {
    let mut v: Vec<(u8, &'static Entry)> = INDEX
        .iter()
        .filter(|e| matches(e, q))
        .map(|e| {
            let label = format!("{} {}", e.label, crate::i18n::tr(e.label)).to_lowercase();
            let in_label = words(q).iter().all(|w| label.contains(w.as_str()));
            (if in_label { 0 } else { 1 }, e)
        })
        .collect();
    v.sort_by_key(|(rank, _)| *rank);
    v.into_iter().map(|(_, e)| e).collect()
}

/// вкладка, у которой есть подходящая запись (для списка вкладок слева)
pub fn tab_has_hit(tab: &str, q: &str) -> bool {
    INDEX.iter().any(|e| e.tab == tab && matches(e, q))
}

const ROW: i32 = 34;

/// страница результатов: строка на настройку, «подпись — вкладка»; щелчок открывает вкладку
pub fn draw_results(ui: &mut Ui, st: &State, b: R) {
    let t = ui.t;
    let q = st.search_field.text.trim().to_string();
    let hits = search(&q);
    let mut y = b.y;
    if hits.is_empty() {
        ui.text_in(PX, R::new(b.x, y, b.w, 26), "ничего не нашлось", t.text_faint);
        ui.text_in(PX, R::new(b.x, y + 28, b.w, 26), "попробуй другое слово: кофе, ночь, часы, звук", t.text_faint);
        return;
    }
    for e in hits.iter().take(40) {
        let r = R::new(b.x, y, b.w, ROW - 4);
        let tab = crate::settings::find(e.tab);
        let tab_name = tab.map(|x| x.name).unwrap_or("");
        let icon = tab.map(|x| x.icon).unwrap_or("gear");
        ui.button(r, "", "", Tone::Ghost, false, true, Act::STab(e.tab));
        ui.cv.icon(icon, r.x + 8, r.y + (r.h - 14) / 2, 1, t.red);
        let tw = ui.w(PX, tab_name).min(r.w / 3);
        ui.text_in(PX, R::new(r.x + 34, r.y, r.w - 34 - tw - 24, r.h), e.label, t.text);
        ui.text_in(PX, R::new(r.right() - tw - 12, r.y, tw, r.h), tab_name, t.text_faint);
        ui.zone(r, Act::STab(e.tab), Cur::Pointer);
        y += ROW;
    }
    let _ = tabs;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// подписи из вызовов toggle/slider/opt_btn/num_btn в файлах вкладок: первая строка после R::new(…)
    fn labels_in(src: &str) -> Vec<String> {
        let mut out = vec![];
        for name in ["toggle(", "slider(", "opt_btn(", "num_btn("] {
            let mut rest = src;
            while let Some(p) = rest.find(name) {
                rest = &rest[p + name.len()..];
                let Some(r) = rest.find("R::new(") else { continue };
                // сначала должен идти R::new — не дальше первых 40 знаков (иначе это чужой вызов)
                if r > 40 {
                    continue;
                }
                let after = &rest[r..];
                let Some(close) = after.find(')') else { continue };
                let tail = after[close + 1..].trim_start_matches([',', ' ']);
                if let Some(s) = tail.strip_prefix('"') {
                    if let Some(e) = s.find('"') {
                        out.push(s[..e].to_string());
                    }
                }
            }
        }
        out
    }

    /// каждая подпись настройки из файлов вкладок есть в указателе: новая настройка без записи ломает тест
    #[test]
    fn the_index_covers_every_setting_label() {
        let files = [
            ("settabs.rs", include_str!("settabs.rs")),
            ("bartab.rs", include_str!("bartab.rs")),
            ("wall.rs", include_str!("wall.rs")),
            ("displays.rs", include_str!("displays.rs")),
            ("login.rs", include_str!("login.rs")),
            ("agents.rs", include_str!("agents.rs")),
            ("games.rs", include_str!("games.rs")),
            ("bttab.rs", include_str!("bttab.rs")),
            ("fonttab.rs", include_str!("fonttab.rs")),
            ("animtab.rs", include_str!("animtab.rs")),
            ("updtab.rs", include_str!("updtab.rs")),
        ];
        let mut missing = vec![];
        for (f, src) in files {
            for l in labels_in(src) {
                if !INDEX.iter().any(|e| e.label == l) {
                    missing.push(format!("{f}: «{l}»"));
                }
            }
        }
        assert!(missing.is_empty(), "settings without an index entry (add them to setindex_data.rs):\n{}", missing.join("\n"));
    }

    /// у каждой записи есть настоящая вкладка
    #[test]
    fn every_entry_points_at_a_real_tab() {
        for e in INDEX {
            assert!(crate::settings::find(e.tab).is_some(), "{}: no tab {}", e.label, e.tab);
        }
    }

    /// по синонимам и по-английски находится нужная вкладка
    #[test]
    fn people_find_things_by_the_words_they_use() {
        let tab_of = |q: &str| search(q).first().map(|e| e.tab);
        assert_eq!(tab_of("caffeine"), Some("comfort"));
        assert_eq!(tab_of("keep awake"), Some("comfort"));
        assert_eq!(tab_of("night light"), Some("comfort"));
        assert_eq!(tab_of("12 hour"), Some("region"));
        assert_eq!(tab_of("hijri"), Some("region"));
        assert_eq!(tab_of("weekend"), Some("region"));
        assert_eq!(tab_of("blur"), Some("look"));
        assert!(search("zzzz nothing").is_empty());
        assert!(search("   ").is_empty());
    }
}
