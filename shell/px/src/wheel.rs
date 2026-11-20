// Колесо и тачпад. Мышь шлёт щелчки (discrete / value120): один щелчок — один шаг, как всегда. Тачпад шлёт много
// мелких сдвигов в пикселях (absolute); раньше каждый такой сдвиг делили на 15 и умножали на шаг щелчка, так что
// несколько миллиметров движения пальца листали страницу, а в шелле каждое событие считалось целым шагом. Теперь
// сдвиги тачпада — в пикселях: список едет за пальцем один к одному, а там, где нужны целые шаги (масштаб, громкость,
// перемотка), пиксели копятся до порога.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scroll {
    /// щелчки колеса (дробные у колёс с высоким разрешением)
    Notches(f64),
    /// плавный сдвиг тачпада, пиксели
    Pixels(f64),
}

/// событие оси → сдвиг; None — нулевое событие. `discrete` — щелчки, `value120` — доли щелчка (120 на щелчок),
/// `absolute` — пиксели плавной прокрутки
pub fn scroll_of(discrete: i32, value120: i32, absolute: f64) -> Option<Scroll> {
    let s = if discrete != 0 {
        Scroll::Notches(discrete as f64)
    } else if value120 != 0 {
        Scroll::Notches(value120 as f64 / 120.0)
    } else {
        Scroll::Pixels(absolute)
    };
    match s {
        Scroll::Notches(n) if n.abs() < 0.01 => None,
        Scroll::Pixels(p) if p.abs() < 0.01 => None,
        s => Some(s),
    }
}

/// сколько пикселей тачпада — один шаг там, где шаги целые
pub const PIXELS_PER_STEP: f64 = 40.0;

/// копилка шагов: пиксели тачпада превращаются в целые шаги, остаток переносится; смена направления или пауза сбрасывают
#[derive(Default)]
pub struct Steps {
    acc: f64,
    at: Option<Instant>,
}

impl Steps {
    /// целые шаги от этого события (со знаком: плюс — вниз); щелчок колеса — сразу его шаги, не меньше одного
    pub fn feed(&mut self, s: Scroll) -> i32 {
        match s {
            Scroll::Notches(n) => {
                self.acc = 0.0;
                (n.signum() * n.abs().round().max(1.0)) as i32
            }
            Scroll::Pixels(p) => {
                if self.at.is_some_and(|t| t.elapsed() > Duration::from_millis(300)) || self.acc * p < 0.0 {
                    self.acc = 0.0;
                }
                self.at = Some(Instant::now());
                self.acc += p / PIXELS_PER_STEP;
                let whole = (self.acc + 1e-6).trunc();
                self.acc -= whole;
                whole as i32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mouse_notch_is_one_step_and_zero_events_are_ignored() {
        assert_eq!(scroll_of(1, 0, 0.0), Some(Scroll::Notches(1.0)));
        assert_eq!(scroll_of(0, -120, 0.0), Some(Scroll::Notches(-1.0)));
        assert_eq!(scroll_of(0, 60, 0.0), Some(Scroll::Notches(0.5)), "a high-resolution wheel's half notch");
        assert_eq!(scroll_of(0, 0, 3.5), Some(Scroll::Pixels(3.5)));
        assert_eq!(scroll_of(0, 0, 0.0), None);
        assert_eq!(scroll_of(0, 0, 0.001), None);
    }

    #[test]
    fn a_touchpad_swipe_adds_up_to_whole_steps_instead_of_one_step_per_event() {
        let mut s = Steps::default();
        // сорок событий по 3 пикселя — 120 пикселей, то есть три шага, а не сорок
        let total: i32 = (0..40).map(|_| s.feed(Scroll::Pixels(3.0))).sum();
        assert_eq!(total, 3);
        assert_eq!(s.feed(Scroll::Pixels(3.0)), 0, "the remainder waits");
    }

    #[test]
    fn a_notch_steps_at_once_and_reversing_forgets_the_leftover() {
        let mut s = Steps::default();
        assert_eq!(s.feed(Scroll::Notches(1.0)), 1);
        assert_eq!(s.feed(Scroll::Notches(-1.0)), -1);
        assert_eq!(s.feed(Scroll::Notches(0.5)), 1, "even half a notch is a step");
        s.feed(Scroll::Pixels(30.0));
        assert_eq!(s.feed(Scroll::Pixels(-30.0)), 0, "the other direction starts from zero, no phantom step");
    }
}
