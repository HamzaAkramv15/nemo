// Имя рабочего стола и его знак: рыбка 16×9 клеток, рисуется заливками (то же, что
// .github/assets/nemo-logo.svg). Знак стоит там, где раньше был иероглиф: экран блокировки, мастер
// первой настройки, «о шелле».

use crate::gfx::{C, R};
use crate::ui::{Action, Ui};

/// как называется рабочий стол на экране. Команды, папки и пакеты остаются yami: на них держатся
/// модули, обновления и конфиги тех, кто пришёл с yami
pub const NAME: &str = "Nemo";

/// версия Nemo. Версия пакетов (CARGO_PKG_VERSION, VERSION) остаётся версией yami, на которой Nemo
/// стоит: от неё зависят обновление, откат и совместимость модулей, и менять её на 0.1 нельзя
pub const VERSION: &str = "0.1.0";

/// клетки рыбки: # контур, W блик, l светлая, t светлая с тенью, b тело, s низ тела
pub const FISH: [&str; 9] = [
    "....#####....##.",
    "..##WWWbW##.#ll#",
    ".#WWlbbbbbb#llt#",
    "#Wllllbbbbbbbl#.",
    "#ll#llbbbbbbbl#.",
    "#tllllbbbbbsblt#",
    ".#ttllbbbss#tll#",
    "..##tssss##.#tt#",
    "....#####....##.",
];

/// номер цвета клетки в `tones`; None — пусто
pub fn tone_index(ch: u8) -> Option<usize> {
    b"#Wltbs".iter().position(|&c| c == ch)
}

/// цвета рыбки по палитре рабочего стола (порядок как в `tone_index`): контур — тёмный акцент,
/// тело — акцент, блики — к яркому. Те же роли, что и в палитре для остальных программ
/// (accent, deep, bright, bg), поэтому знак меняет цвет вместе с темой; в fastfetch — тоже
pub fn tones(accent: C, deep: C, bright: C, bg: C) -> [C; 6] {
    [deep.lerp(bg, 0.35), bright, bright.lerp(accent, 0.4), bright.lerp(accent, 0.6), accent, accent.lerp(deep, 0.5)]
}

/// размер знака в пикселях при клетке `cell`
pub fn fish_size(cell: i32) -> (i32, i32) {
    (16 * cell, 9 * cell)
}

/// нарисовать рыбку цветами темы; `fade` — 1.0 полный цвет, меньше — бледнее (как у знака на
/// экране блокировки)
pub fn fish<A: Action>(ui: &mut Ui<A>, x: i32, y: i32, cell: i32, fade: f32) {
    let t = ui.t;
    let c = tones(t.red, t.blood, t.ember, t.bg);
    for (j, row) in FISH.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if let Some(k) = tone_index(ch) {
                ui.cv.fill(R::new(x + i as i32 * cell, y + j as i32 * cell, cell, cell), if fade >= 1.0 { c[k] } else { c[k].fade(fade) });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// знак — ровно 16×9, и каждая клетка либо пустая, либо из шести цветов
    #[test]
    fn the_fish_is_a_clean_grid() {
        assert_eq!(super::FISH.len(), 9);
        for row in super::FISH {
            assert_eq!(row.len(), 16, "{row}");
            for b in row.bytes() {
                assert!(b == b'.' || super::tone_index(b).is_some(), "{}", b as char);
            }
        }
    }

    /// другая тема — другие цвета, и тело рыбки — акцент темы
    #[test]
    fn the_fish_wears_the_palette() {
        use crate::gfx::C;
        let a = super::tones(C::hex(0xc4161c), C::hex(0x5a0b0e), C::hex(0xff6a60), C::hex(0x0e0c0c));
        let b = super::tones(C::hex(0x3a9fd0), C::hex(0x0b3a52), C::hex(0x7fd0ff), C::hex(0x0c0e10));
        assert_ne!(a, b);
        assert_eq!(a[4], C::hex(0xc4161c));
    }
}
