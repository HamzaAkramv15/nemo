// Текст справа налево: арабские буквы получают нужное начертание (отдельная, начальная, средняя,
// конечная; лам+алеф — одна лигатура), а строка переставляется в видимый порядок по алгоритму
// Unicode (UAX #9), чтобы её можно было рисовать слева направо, символ за символом, как любую другую.
// Это вся поддержка: движок текста не знает ни про шрифтовую вёрстку, ни про GSUB. Для арабского
// языка интерфейса этого хватает (буквы, цифры, латиница в одной строке, скобки); засечки над
// буквами (огласовки) остаются при своей букве. Строка без знаков справа налево возвращается как
// есть, без копирования: остальные языки ничего не платят.

use crate::rtl_table::{FORMS, LAM_ALEF};
use std::borrow::Cow;

fn is_rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

static DIGITS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// арабско-индийские цифры (٠١٢٣) вместо 0123 во всём выводе; по выбору пользователя
pub fn set_digits(on: bool) {
    DIGITS.store(on, std::sync::atomic::Ordering::Relaxed);
}

fn eastern(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_digit() { char::from_u32(0x0660 + (c as u32 - '0' as u32)).unwrap_or(c) } else { c }).collect()
}

pub fn has_rtl(s: &str) -> bool {
    s.chars().any(is_rtl_char)
}

/// первая «сильная» буква строки — справа налево: тогда и строка читается справа налево
pub fn base_rtl(s: &str) -> bool {
    for c in s.chars() {
        if is_rtl_char(c) && c.is_alphabetic() {
            return true;
        }
        if c.is_alphabetic() {
            return false;
        }
    }
    false
}

/// знак над буквой или под ней: стоит при букве и не меняет соединения
fn mark(c: char) -> bool {
    matches!(c as u32, 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7 | 0x06E8 | 0x06EA..=0x06ED)
}

fn row(c: char) -> Option<&'static (u32, u32, u32, u32, u32)> {
    FORMS.binary_search_by_key(&(c as u32), |r| r.0).ok().map(|i| &FORMS[i])
}

/// соединяется ли буква со следующей (у неё есть начальная форма) и с предыдущей (есть конечная)
fn joins_next(c: char) -> bool {
    c == '\u{0640}' || row(c).is_some_and(|r| r.3 != 0)
}
fn joins_prev(c: char) -> bool {
    c == '\u{0640}' || row(c).is_some_and(|r| r.2 != 0)
}

/// буквам даются формы по соседям; порядок остаётся логическим
fn shape(s: &str) -> Vec<char> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::with_capacity(cs.len());
    // ближайшая буква слева/справа, мимо знаков
    let prev_of = |i: usize| (0..i).rev().map(|j| cs[j]).find(|c| !mark(*c));
    let next_of = |i: usize| cs[i + 1..].iter().copied().find(|c| !mark(*c));
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        let Some(r) = row(c).copied() else {
            out.push(c);
            i += 1;
            continue;
        };
        let p_ok = prev_of(i).is_some_and(joins_next) && joins_prev(c);
        // лам + алеф без знака между ними: одна лигатура
        if c == '\u{0644}' {
            if let Some(&n) = cs.get(i + 1) {
                if let Some(l) = LAM_ALEF.iter().find(|l| l.0 == n as u32) {
                    out.push(char::from_u32(if p_ok { l.2 } else { l.1 }).unwrap_or(c));
                    i += 2;
                    continue;
                }
            }
        }
        let n_ok = joins_next(c) && next_of(i).is_some_and(joins_prev);
        let form = match (p_ok, n_ok) {
            (true, true) => r.4,
            (false, true) => r.3,
            (true, false) => r.2,
            (false, false) => r.1,
        };
        out.push(char::from_u32(if form == 0 { r.1 } else { form }).unwrap_or(c));
        i += 1;
    }
    out
}

/// зеркальный знак для скобок в участке справа налево
fn mirror(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        _ => c,
    }
}

/// строка в видимом порядке слева направо, с начертаниями букв. Без знаков справа налево — она сама
pub fn visual(s: &str) -> Cow<'_, str> {
    visual_with(s, DIGITS.load(std::sync::atomic::Ordering::Relaxed))
}

fn visual_with(s: &str, digits: bool) -> Cow<'_, str> {
    if digits && s.bytes().any(|b| b.is_ascii_digit()) {
        let e = eastern(s);
        return Cow::Owned(visual_text(&e).into_owned());
    }
    visual_text(s)
}

fn visual_text(s: &str) -> Cow<'_, str> {
    if !has_rtl(s) {
        return Cow::Borrowed(s);
    }
    use unicode_bidi::{BidiInfo, Level};
    let shaped: String = shape(s).into_iter().collect();
    // уровень строки — по её первой сильной букве, как у читателя
    let info = BidiInfo::new(&shaped, if base_rtl(s) { Some(Level::rtl()) } else { Some(Level::ltr()) });
    let mut out = String::with_capacity(shaped.len());
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let chars: Vec<char> = shaped[run.clone()].chars().collect();
            if levels[run.start].is_rtl() {
                // справа налево: буквы в обратном порядке, но знаки остаются после своей буквы
                let mut clusters: Vec<Vec<char>> = vec![];
                for c in chars {
                    match clusters.last_mut() {
                        Some(last) if mark(c) => last.push(c),
                        _ => clusters.push(vec![c]),
                    }
                }
                for cl in clusters.into_iter().rev() {
                    out.extend(cl.into_iter().map(mirror));
                }
            } else {
                out.extend(chars);
            }
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cps(s: &str) -> Vec<u32> {
        s.chars().map(|c| c as u32).collect()
    }

    #[test]
    fn text_without_rtl_is_untouched_and_not_copied() {
        for s in ["", "plain text 123", "Русский текст", "日本語"] {
            assert!(matches!(visual(s), Cow::Borrowed(_)), "{s}");
        }
        assert!(!has_rtl("abc") && has_rtl("abc ع"));
    }

    #[test]
    fn letters_take_the_form_their_neighbours_give_them() {
        // «سلام»: سـ (начальная) + لا (лигатура, конечная) + م (отдельная); в видимом порядке справа налево
        assert_eq!(cps(&visual("سلام")), vec![0xFEE1, 0xFEFC, 0xFEB3]);
        // «محمد»: м начальная, х средняя (ح), м средняя, д конечная; видимый порядок обратный
        assert_eq!(cps(&visual("محمد")), vec![0xFEAA, 0xFEE4, 0xFEA4, 0xFEE3]);
        // одна буква — отдельная форма; хамза не соединяется ни с кем
        assert_eq!(cps(&visual("ب")), vec![0xFE8F]);
        assert_eq!(cps(&visual("ءا")), vec![0xFE8D, 0xFE80]);
    }

    #[test]
    fn lam_alef_is_one_glyph_alone_or_after_a_joiner() {
        assert_eq!(cps(&visual("لا")), vec![0xFEFB]);
        assert_eq!(cps(&visual("بلا")), vec![0xFEFC, 0xFE91]);
    }

    #[test]
    fn digits_and_latin_keep_their_order_inside_arabic() {
        // «من 12 ملف»: слова справа налево, число 12 остаётся «12»
        let v = visual("من 12 ملف");
        assert!(v.contains("12"), "{v}");
        // «Nemo ملف»: строка по первой букве слева направо: Nemo первым
        let v = visual("Nemo ملف");
        assert!(v.starts_with("Nemo"), "{v}");
        // арабская строка начинается справа: латиница оказывается левее арабских слов
        let v = visual("ملف Nemo");
        assert!(v.ends_with("ملف") || v.chars().last().is_some_and(is_rtl_char), "{v}");
    }

    #[test]
    fn marks_stay_with_their_letter() {
        // باَ: фатха после ба остаётся после неё, а не уезжает в начало
        let v = cps(&visual("بَا"));
        let fatha = v.iter().position(|c| *c == 0x064E).expect("mark kept");
        assert!(fatha > 0 && (v[fatha - 1] == 0xFE91 || v[fatha - 1] == 0xFE90 || v[fatha - 1] == 0xFE8F), "{v:x?}");
    }

    #[test]
    fn brackets_are_mirrored_in_rtl_runs() {
        let v = visual("(ب)");
        assert_eq!(v.chars().next(), Some('('), "{v}");
        assert_eq!(v.chars().last(), Some(')'), "{v}");
    }

    #[test]
    fn base_direction_follows_the_first_letter() {
        assert!(base_rtl("ملف"));
        assert!(base_rtl("123 ملف"));
        assert!(!base_rtl("file ملف"));
        assert!(!base_rtl("123"));
    }

    #[test]
    fn eastern_digits_are_opt_in() {
        assert_eq!(eastern("17:27 ٣"), "١٧:٢٧ ٣");
        assert_eq!(visual_with("12", false), "12");
        assert_eq!(visual_with("12 ab", true), "١٢ ab");
        assert!(matches!(visual_with("no digits", true), Cow::Borrowed(_)));
    }
}
