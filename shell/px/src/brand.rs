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

/// Рыбка: один гладкий силуэт (тело-линза, раздвоенный хвост, один глаз-дыра) на холсте 256 единиц,
/// тот же, что в .github/assets/nemo-logo.svg. Рисуется сама, с мягкими краями, без картинок и без
/// зависимостей; цвет — акцент рабочего стола. Размер — клетки: 16 по ширине и 10 по высоте
/// кубическая кривая Безье: четыре точки
type Cubic = ((f32, f32), (f32, f32), (f32, f32), (f32, f32));
const BODY: [Cubic; 2] = [((242.0, 128.0), (204.0, 58.0), (124.0, 56.0), (76.0, 108.0)), ((76.0, 148.0), (124.0, 200.0), (204.0, 198.0), (242.0, 128.0))];
const TAIL: [(f32, f32); 4] = [(26.0, 70.0), (46.0, 128.0), (26.0, 186.0), (76.0, 148.0)];
const EYE: (f32, f32, f32) = (190.0, 112.0, 10.0);
/// ширина силуэта на холсте и левый край: из них масштаб
const SRC_W: f32 = 224.0;
const SRC_X: f32 = 24.0;

/// многоугольник силуэта: кривые разбиты на отрезки
fn outline() -> Vec<(f32, f32)> {
    let cubic = |p: &Cubic, t: f32| {
        let u = 1.0 - t;
        let f = |a: f32, b: f32, c: f32, d: f32| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
        (f(p.0 .0, p.1 .0, p.2 .0, p.3 .0), f(p.0 .1, p.1 .1, p.2 .1, p.3 .1))
    };
    let mut v = vec![];
    for i in 0..=24 {
        v.push(cubic(&BODY[0], i as f32 / 24.0));
    }
    v.extend(TAIL);
    for i in 1..=24 {
        v.push(cubic(&BODY[1], i as f32 / 24.0));
    }
    v
}

fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut c = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
        j = i;
    }
    c
}

/// покрытие рыбки в коробке w×h пикселей, 0..255 (4×4 выборки на пиксель, глаз — дыра); рыбка по ширине
/// на всю коробку и по высоте посередине
pub fn mask(w: i32, h: i32) -> Vec<u8> {
    let poly = outline();
    let s = w as f32 / SRC_W;
    let mut out = vec![0u8; (w.max(0) * h.max(0)) as usize];
    for j in 0..h {
        for i in 0..w {
            let mut n = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = (i as f32 + (sx as f32 + 0.5) / 4.0) / s + SRC_X;
                    let y = (j as f32 + (sy as f32 + 0.5) / 4.0 - h as f32 / 2.0) / s + 128.0;
                    let (dx, dy) = (x - EYE.0, y - EYE.1);
                    if inside(&poly, x, y) && dx * dx + dy * dy > EYE.2 * EYE.2 {
                        n += 1;
                    }
                }
            }
            out[(j * w + i) as usize] = (n * 255 / 16) as u8;
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
    (16 * cell, 10 * cell)
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
    /// силуэт — это рыбка: тело в середине, хвост слева, пусто по углам, глаз — дыра
    #[test]
    fn the_fish_is_a_fish() {
        let (w, h) = (160, 100);
        let m = super::mask(w, h);
        let at = |x: i32, y: i32| m[(y * w + x) as usize];
        assert_eq!(at(0, 0), 0, "corner is empty");
        assert_eq!(at(w - 1, h - 1), 0);
        assert!(at(w / 2 + 6, h / 2) > 250, "the body is solid");
        assert!(at(10, 25) > 200, "the tail is on the left");
        let s = w as f32 / super::SRC_W;
        let (ex, ey) = (((super::EYE.0 - super::SRC_X) * s) as i32, ((super::EYE.1 - 128.0) * s + h as f32 / 2.0) as i32);
        assert_eq!(at(ex, ey), 0, "the eye is a hole");
        let solid = m.iter().filter(|a| **a > 128).count() as f32 / (w * h) as f32;
        assert!((0.35..0.65).contains(&solid), "{solid:.2} of the box is fish");
    }

    /// сверху и снизу рыбка одинакова, кроме глаза: ось симметрии — середина коробки
    #[test]
    fn the_fish_is_symmetric_about_its_axis() {
        let (w, h) = (96, 60);
        let m = super::mask(w, h);
        let mut diff = 0;
        for j in 0..h / 2 {
            for i in 0..w {
                let (a, b) = (m[(j * w + i) as usize] as i32, m[((h - 1 - j) * w + i) as usize] as i32);
                if (a - b).abs() > 60 {
                    diff += 1;
                }
            }
        }
        // отличается только место вокруг глаза
        assert!(diff < 80, "{diff} pixels differ");
    }

    #[test]
    fn the_size_matches_the_cells() {
        assert_eq!(super::fish_size(4), (64, 40));
        assert_eq!(super::mask(32, 20).len(), 640);
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
