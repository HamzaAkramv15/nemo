// Эмодзи и знаки: запрос «:улыбка» или «:smile» в лаунчере. Данные — аннотации Unicode CLDR (английские и
// арабские названия и слова-подсказки), data/emoji.tsv, зашиты в бинарь (около 350 КБ), без сети и без
// новых зависимостей. Разбирается построчно при каждом запросе: 2 300 строк — доли миллисекунды.

const DATA: &str = include_str!("../data/emoji.tsv");

pub struct Emoji {
    pub ch: &'static str,
    pub en: &'static str,
    en_kw: &'static str,
    pub ar: &'static str,
    ar_kw: &'static str,
}

fn rows() -> impl Iterator<Item = Emoji> {
    DATA.lines().filter_map(|l| {
        let mut f = l.split('\t');
        Some(Emoji { ch: f.next()?, en: f.next()?, en_kw: f.next().unwrap_or(""), ar: f.next().unwrap_or(""), ar_kw: f.next().unwrap_or("") })
    })
}

/// найденные по запросу: все слова должны встретиться в названии или словах-подсказках (английских или
/// арабских); выше те, где слово — начало названия, потом само название, потом подсказки
pub fn search(q: &str, limit: usize) -> Vec<Emoji> {
    let words: Vec<String> = q.to_lowercase().split_whitespace().map(str::to_string).collect();
    if words.is_empty() {
        return vec![];
    }
    let mut hits: Vec<(u8, Emoji)> = vec![];
    for e in rows() {
        let name = format!("{} {}", e.en, e.ar).to_lowercase();
        let kw = format!("{}|{}", e.en_kw, e.ar_kw).to_lowercase();
        if !words.iter().all(|w| name.contains(w.as_str()) || kw.contains(w.as_str())) {
            continue;
        }
        let rank = if words.iter().all(|w| e.en.to_lowercase().starts_with(w.as_str()) || e.ar.starts_with(w.as_str())) {
            0
        } else if words.iter().all(|w| name.contains(w.as_str())) {
            1
        } else {
            2
        };
        hits.push((rank, e));
    }
    hits.sort_by_key(|(r, _)| *r);
    hits.into_iter().take(limit).map(|(_, e)| e).collect()
}

/// название на языке интерфейса: арабское при арабском, иначе английское
pub fn name(e: &Emoji) -> &'static str {
    if crate::i18n::code() == "ar" && !e.ar.is_empty() {
        e.ar
    } else {
        e.en
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_is_clean_and_covers_the_basics() {
        let n = rows().count();
        assert!(n > 2000, "{n} rows");
        for e in rows() {
            assert!(!e.ch.is_empty() && !e.en.is_empty(), "{:?}", e.ch);
            assert!(!e.ch.chars().any(|c| ('\u{1F3FB}'..='\u{1F3FF}').contains(&c)), "skin tone variants are left out: {}", e.ch);
        }
    }

    #[test]
    fn people_find_emoji_by_the_words_they_know() {
        assert_eq!(search("grinning face", 1)[0].ch, "😀");
        assert!(search("fish", 20).iter().any(|e| e.ch == "🐟"));
        assert!(search("flag egypt", 5).iter().any(|e| e.ch == "🇪🇬"));
        assert!(search("heart", 40).iter().any(|e| e.ch == "❤"), "red heart");
        // арабскими словами
        assert!(!search("سمكة", 10).is_empty(), "Arabic names are searchable");
        assert!(search("zzzzqq", 10).is_empty() && search("  ", 10).is_empty());
    }

    #[test]
    fn a_name_that_starts_with_the_word_comes_before_a_keyword_hit() {
        let r = search("fish", 50);
        let first_kw = r.iter().position(|e| !e.en.to_lowercase().starts_with("fish")).unwrap_or(r.len());
        assert!(r.iter().take(first_kw).all(|e| e.en.to_lowercase().starts_with("fish")));
        assert!(first_kw >= 1);
    }
}
