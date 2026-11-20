// Имя рабочего стола и его знак: рыбка 16×9 клеток, рисуется заливками (то же, что
// .github/assets/nemo-logo.svg). Знак стоит там, где раньше был иероглиф: экран блокировки, мастер
// первой настройки, «о шелле».

use crate::gfx::{C, R};
use crate::ui::{Action, Ui};

/// как называется рабочий стол на экране. Команды, папки и пакеты остаются nemo: на них держатся
/// модули, обновления и конфиги тех, кто пришёл с nemo
pub const NAME: &str = "Nemo";

/// версия Nemo. Версия пакетов (CARGO_PKG_VERSION, VERSION) остаётся версией nemo, на которой Nemo
/// стоит: от неё зависят обновление, откат и совместимость модулей, и менять её на 0.1 нельзя
pub const VERSION: &str = "0.1.0";

/// Рыбка: пиксельная, 16×8 клеток (решётка ниже; `#` — тело, `.` — пусто, глаз — дыра), та же, что в
/// .github/assets/logo/nemo-fish.svg. Рисуется сама, заливками, без картинок и без зависимостей; цвет —
/// акцент рабочего стола. Клетка — целое число пикселей, поэтому края чёткие
const GRID: [&str; 8] = [
    "....#####.....##",
    "..########....##",
    ".##########..###",
    "##.############.",
    "##.############.",
    ".##########..###",
    "..########....##",
    "....#####.....##",
];

/// покрытие рыбки в коробке w×h пикселей: 0 или 255; рыбка по ширине на всю коробку (клетка w/16) и по высоте
/// посередине. При клетке не в целое число пикселей клетки выходят неровными, поэтому берите w кратным 16
pub fn mask(w: i32, h: i32) -> Vec<u8> {
    let cell = w as f32 / 16.0;
    let top = (h as f32 - 8.0 * cell) / 2.0;
    let mut out = vec![0u8; (w.max(0) * h.max(0)) as usize];
    for j in 0..h {
        for i in 0..w {
            let gx = ((i as f32 + 0.5) / cell) as usize;
            let gy = ((j as f32 + 0.5 - top) / cell).floor();
            if gy >= 0.0 && (gy as usize) < 8 && gx < 16 && GRID[gy as usize].as_bytes()[gx] == b'#' {
                out[(j * w + i) as usize] = 255;
            }
        }
    }
    out
}

/// цвета рыбки по палитре рабочего стола (порядок как в `tone_index`): контур — тёмный акцент,
/// тело — акцент, блики — к яркому. Те же роли, что и в палитре для остальных программ
/// (accent, deep, bright, bg), поэтому знак меняет цвет вместе с темой; в fastfetch — тоже
pub fn tones(accent: C, deep: C, bright: C, bg: C) -> [C; 6] {
    [deep.lerp(bg, 0.35), bright, bright.lerp(accent, 0.4), bright.lerp(accent, 0.6), accent, accent.lerp(deep, 0.5)]
}

/// размер знака в пикселях при клетке `cell`
pub fn fish_size(cell: i32) -> (i32, i32) {
    (16 * cell, 8 * cell)
}

/// размер коробки и покрытие
type Mask = ((i32, i32), Vec<u8>);

thread_local! {
    /// последняя маска: знак рисуется каждый кадр, а считать её дорого
    static CACHE: std::cell::RefCell<Option<Mask>> = const { std::cell::RefCell::new(None) };
}

/// нарисовать рыбку цветом акцента; `fade` — 1.0 полный цвет, меньше — бледнее (как у знака на экране
/// блокировки)
pub fn fish<A: Action>(ui: &mut Ui<A>, x: i32, y: i32, cell: i32, fade: f32) {
    let (w, h) = fish_size(cell);
    let col = ui.t.red;
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().map(|m| m.0) != Some((w, h)) {
            *c = Some(((w, h), mask(w, h)));
        }
        let m = &c.as_ref().map(|m| &m.1).expect("just filled");
        for j in 0..h {
            for i in 0..w {
                let a = m[(j * w + i) as usize];
                if a > 0 {
                    ui.cv.fill(R::new(x + i, y + j, 1, 1), col.alpha(a as f32 / 255.0 * fade.min(1.0)));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    /// силуэт — это рыбка: тело в середине, хвост справа, пусто по углам, глаз — дыра
    #[test]
    fn the_fish_is_a_fish() {
        let (w, h) = (160, 80);
        let m = super::mask(w, h);
        let at = |x: i32, y: i32| m[(y * w + x) as usize];
        assert_eq!(at(0, 0), 0, "corner is empty");
        assert_eq!(at(w - 1, h / 2), 0, "the tail has a notch");
        assert_eq!(at(w / 2, h / 2), 255, "the body is solid");
        assert_eq!(at(w - 5, 5), 255, "the tail tip is on the right");
        assert_eq!(at(25, h / 2 - 5), 0, "the eye is a hole");
        assert_eq!(at(5, h / 2 - 5), 255, "the nose is in front of the eye");
    }

    /// сверху и снизу рыбка одинакова, кроме глаза (он на два ряда посередине): решётка симметрична
    #[test]
    fn the_fish_is_symmetric_about_its_axis() {
        for j in 0..4 {
            assert_eq!(super::GRID[j], super::GRID[7 - j], "row {j}");
        }
        assert!(super::GRID.iter().all(|r| r.len() == 16));
    }

    #[test]
    fn the_size_matches_the_cells() {
        assert_eq!(super::fish_size(4), (64, 32));
        assert_eq!(super::mask(32, 16).len(), 512);
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
