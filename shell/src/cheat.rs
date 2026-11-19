// Шпаргалка по клавишам: окно со всеми биндами (свои и встроенные, с текущими сочетаниями из настроек),
// строка поиска вверху — печатай «обои», «mod+t», «скриншот», остаётся подходящее. Открывается
// командой `nemo msg cheat` (и клавишей Mod+F1 по умолчанию), закрывается Esc. Вкладка «клавиши» в
// настройках остаётся для переназначения; здесь — только посмотреть быстро.

use crate::app::{App, SKind, State};
use crate::font::PX;
use crate::gfx::{BoxStyle, R};
use crate::keys::{self, BINDS, CATS};
use crate::ui::{Act, Cur, Field, Ui};
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym};

pub const SIZE: (i32, i32) = (1040, 580);
const ROW: i32 = 28;

#[derive(Default)]
pub struct Cheat {
    pub field: Field,
    /// сколько строк уехало вверх
    pub scroll: i32,
}

pub enum Row {
    Head(String),
    Bind { title: String, key: String },
}

/// все строки: категории по порядку, внутри — бинды с сочетанием; свои команды — в конце
pub fn rows(st: &State, query: &str) -> Vec<Row> {
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
    let hit = |hay: &str| words.iter().all(|w| hay.to_lowercase().contains(w.as_str()));
    let mut out = vec![];
    for (id, ru, en) in CATS {
        let cat_name = crate::i18n::select(ru, en).to_string();
        let mut items: Vec<Row> = vec![];
        if *id == "custom" {
            for c in &st.set.custom_binds {
                if !c.key.is_empty() && hit(&format!("{} {} {cat_name}", c.cmd, c.key)) {
                    items.push(Row::Bind { title: c.cmd.clone(), key: c.key.clone() });
                }
            }
        } else {
            for b in BINDS.iter().filter(|b| b.cat == *id) {
                let key = keys::key_of(&st.set, b);
                let title = keys::title_of(b).to_string();
                if !key.is_empty() && hit(&format!("{title} {} {} {key} {cat_name}", b.ru, b.en)) {
                    items.push(Row::Bind { title, key });
                }
            }
        }
        if !items.is_empty() {
            out.push(Row::Head(cat_name));
            out.extend(items);
        }
    }
    out
}

pub fn draw(ui: &mut Ui, st: &State, w: i32, h: i32) {
    let t = ui.t;
    let fr = R::new(0, 0, w - 6, h - 6);
    ui.frame(fr, false);
    ui.header(R::new(1, 1, fr.w - 2, 30), "keys", "горячие клавиши", "");
    ui.ghost(R::new(fr.right() - 28, 5, 24, 22), "close", Act::CloseModal);
    ui.field(R::new(14, 40, fr.w - 28, 28), &st.cheat.field, "поиск: обои, mod+t, скриншот", true, false, Act::Block);
    let list = R::new(14, 78, fr.w - 28, fr.h - 78 - 38);
    let all = rows(st, &st.cheat.field.text);
    if all.is_empty() {
        ui.text_in(PX, R::new(list.x, list.y, list.w, 26), "ничего не нашлось", t.text_faint);
    }
    // две колонки: сначала первая половина, потом вторая
    let per = (all.len() as i32 + 1) / 2;
    let vis = list.h / ROW;
    let colw = (list.w - 20) / 2;
    ui.cv.push_clip(list);
    for (n, row) in all.iter().enumerate() {
        let n = n as i32;
        let (col, i) = if n < per { (0, n) } else { (1, n - per) };
        let y = list.y + (i - st.cheat.scroll) * ROW;
        if i < st.cheat.scroll || y + ROW > list.bottom() + 2 {
            continue;
        }
        let x = list.x + col * (colw + 20);
        match row {
            Row::Head(name) => {
                ui.cv.fill(R::new(x, y + 11, 4, 4), t.red.alpha(0.7));
                ui.text_in(PX, R::new(x + 10, y, colw - 10, ROW - 2), name, t.text_faint);
            }
            Row::Bind { title, key } => {
                let kw = (ui.w(PX, key) + 16).min(colw / 2);
                ui.text_in(PX, R::new(x + 6, y, colw - kw - 18, ROW - 2), title, t.text);
                let chip = R::new(x + colw - kw, y + 2, kw, ROW - 6);
                ui.cv.pxbox(chip, &BoxStyle::new(t.surface, t.border));
                ui.text_center(PX, chip, key, t.text_dim);
            }
        }
    }
    ui.cv.pop_clip();
    ui.zone(list, Act::CheatScroll, Cur::Default);
    let _ = vis;
    ui.text_in(PX, R::new(14, fr.h - 30, fr.w - 28, 22), "Esc — закрыть · стрелки — прокрутка · переназначить: настройки → клавиши", t.text_faint);
}

impl App {
    pub fn cheat_open(&mut self) {
        self.st.cheat = Cheat::default();
    }

    pub fn cheat_scroll(&mut self, d: i32) {
        let n = rows(&self.st, &self.st.cheat.field.text).len() as i32;
        let per = (n + 1) / 2;
        let vis = (SIZE.1 - 6 - 78 - 38) / ROW;
        self.st.cheat.scroll = (self.st.cheat.scroll + d).clamp(0, (per - vis).max(0));
        self.dirty(&SKind::Modal);
    }

    pub fn cheat_key(&mut self, e: &KeyEvent) {
        match e.keysym {
            Keysym::Escape => self.close_modal(),
            Keysym::Down => self.cheat_scroll(1),
            Keysym::Up => self.cheat_scroll(-1),
            Keysym::Page_Down => self.cheat_scroll(8),
            Keysym::Page_Up => self.cheat_scroll(-8),
            _ => {
                let mods = self.mods;
                if App::edit_field(&mut self.st.cheat.field, e, &mods) {
                    self.st.cheat.scroll = 0;
                }
                self.dirty(&SKind::Modal);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st() -> State {
        State::new(crate::config::Settings::default(), Default::default())
    }

    #[test]
    fn every_category_with_binds_is_listed_and_search_narrows() {
        let s = st();
        let all = rows(&s, "");
        let heads = all.iter().filter(|r| matches!(r, Row::Head(_))).count();
        assert!(heads >= 5, "{heads} categories");
        let n_all = all.len();
        let some = rows(&s, "lock");
        assert!(!some.is_empty() && some.len() < n_all);
        assert!(rows(&s, "zzzzqq").is_empty());
    }

    #[test]
    fn a_bind_is_found_by_its_key_and_by_its_name() {
        let s = st();
        let by_key = rows(&s, "mod+t");
        assert!(by_key.iter().any(|r| matches!(r, Row::Bind { key, .. } if key.eq_ignore_ascii_case("mod+t"))));
        assert!(rows(&s, "launcher").iter().any(|r| matches!(r, Row::Bind { .. })));
    }

    #[test]
    fn binds_without_a_key_are_not_shown() {
        let s = st();
        assert!(rows(&s, "").iter().all(|r| match r {
            Row::Bind { key, .. } => !key.is_empty(),
            Row::Head(_) => true,
        }));
    }
}
