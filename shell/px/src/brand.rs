// Имя рабочего стола и его знак: рыбка 16×9 клеток, рисуется заливками (то же, что
// .github/assets/nemo-logo.svg). Знак стоит там, где раньше был иероглиф: экран блокировки, мастер
// первой настройки, «о шелле».

use crate::gfx::{C, R};
use crate::ui::{Action, Ui};

/// как называется рабочий стол на экране. Команды, папки и пакеты остаются yami: на них держатся
/// модули, обновления и конфиги тех, кто пришёл с yami
pub const NAME: &str = "Nemo";

/// клетки рыбки: # контур, W блик, l светлая, t светлая с тенью, b тело, s низ тела
const FISH: [&str; 9] = [
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

fn tone(ch: u8) -> Option<C> {
    Some(C::hex(match ch {
        b'#' => 0x5a5858,
        b'W' => 0xeef1f4,
        b'l' => 0xc8d6e0,
        b'b' => 0x9eb4c6,
        b's' => 0x8fa8b8,
        b't' => 0xaac4d0,
        _ => return None,
    }))
}

/// размер знака в пикселях при клетке `cell`
pub fn fish_size(cell: i32) -> (i32, i32) {
    (16 * cell, 9 * cell)
}

/// нарисовать рыбку; `fade` — 1.0 полный цвет, меньше — бледнее (как у знака на экране блокировки)
pub fn fish<A: Action>(ui: &mut Ui<A>, x: i32, y: i32, cell: i32, fade: f32) {
    for (j, row) in FISH.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if let Some(c) = tone(ch) {
                ui.cv.fill(R::new(x + i as i32 * cell, y + j as i32 * cell, cell, cell), if fade >= 1.0 { c } else { c.fade(fade) });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    /// знак — ровно 16×9, и каждая клетка либо пустая, либо из палитры
    #[test]
    fn the_fish_is_a_clean_grid() {
        assert_eq!(super::FISH.len(), 9);
        for row in super::FISH {
            assert_eq!(row.len(), 16, "{row}");
            for b in row.bytes() {
                assert!(b == b'.' || super::tone(b).is_some(), "{}", b as char);
            }
        }
    }
}
