// Обычные значки: векторные, со сглаживанием, вместо пиксельных 7×7. Каждый значок — несколько
// фигур в квадрате 24×24 (линии, дуги, кольца, круги, скруглённые прямоугольники, многоугольники);
// растр считается по расстоянию до фигуры (SDF), так что края гладкие на любом размере. Значка нет
// в таблице — рисуется прежний пиксельный (gfx::icon), поэтому набор можно пополнять по одному.

/// фигура в координатах 24×24; w — толщина обводки (в тех же единицах)
#[derive(Clone, Copy)]
pub enum P {
    /// отрезок x1 y1 x2 y2 w
    L(f32, f32, f32, f32, f32),
    /// дуга cx cy r от a0 до a1 градусов по часовой (0° — вправо, 90° — вниз), w
    A(f32, f32, f32, f32, f32, f32),
    /// кольцо cx cy r w
    C(f32, f32, f32, f32),
    /// круг cx cy r
    D(f32, f32, f32),
    /// контур скруглённого прямоугольника x y w h r, толщина
    R(f32, f32, f32, f32, f32, f32),
    /// залитый скруглённый прямоугольник x y w h r
    F(f32, f32, f32, f32, f32),
    /// залитый многоугольник
    T(&'static [(f32, f32)]),
    /// вырез кругом cx cy r (убирает всё, что нарисовано до него)
    X(f32, f32, f32),
}
use P::*;

const SPEAKER: &[(f32, f32)] = &[(3.0, 9.0), (7.0, 9.0), (12.0, 4.5), (12.0, 19.5), (7.0, 15.0), (3.0, 15.0)];
const BOLT: &[(f32, f32)] = &[(13.5, 2.5), (5.0, 13.5), (11.0, 13.5), (10.0, 21.5), (19.0, 10.0), (13.0, 10.0)];
const PLAY: &[(f32, f32)] = &[(7.0, 4.0), (20.0, 12.0), (7.0, 20.0)];
const NEXT: &[(f32, f32)] = &[(5.0, 5.0), (15.0, 12.0), (5.0, 19.0)];
const PREV: &[(f32, f32)] = &[(19.0, 5.0), (9.0, 12.0), (19.0, 19.0)];
const DROP_TIP: &[(f32, f32)] = &[(12.0, 3.0), (6.3, 13.0), (17.7, 13.0)];
const LEAF: &[(f32, f32)] = &[(5.0, 19.0), (5.0, 13.0), (10.0, 6.0), (19.0, 4.0), (19.0, 12.0), (12.0, 18.0)];
const ARROW_R: &[(f32, f32)] = &[(21.5, 12.0), (16.5, 7.5), (16.5, 16.5)];
const BELL_BODY: &[(f32, f32)] = &[(6.0, 10.0), (18.0, 10.0), (19.5, 16.0), (4.5, 16.0)];
const REBOOT_HEAD: &[(f32, f32)] = &[(16.0, 2.5), (21.5, 6.5), (15.0, 8.5)];

/// фигуры значка по имени
pub fn shapes(name: &str) -> Option<Vec<P>> {
    let w = 2.5;
    let mut v: Vec<P> = vec![];
    match name {
        "speaker" => v.extend([T(SPEAKER), A(12.0, 12.0, 5.5, -48.0, 48.0, w), A(12.0, 12.0, 9.5, -48.0, 48.0, w)]),
        "mute" => v.extend([T(SPEAKER), L(16.0, 9.0, 22.0, 15.0, w), L(22.0, 9.0, 16.0, 15.0, w)]),
        "mic" => v.extend([F(9.0, 3.0, 6.0, 12.0, 3.0), A(12.0, 11.0, 7.0, 0.0, 180.0, w), L(12.0, 18.0, 12.0, 21.0, w), L(8.0, 21.0, 16.0, 21.0, w)]),
        "wifi" => v.extend([A(12.0, 19.5, 5.5, -138.0, -42.0, w), A(12.0, 19.5, 10.0, -138.0, -42.0, w), A(12.0, 19.5, 14.5, -138.0, -42.0, w), D(12.0, 19.5, 1.9)]),
        "net" => v.extend([L(8.0, 19.0, 8.0, 5.0, w), L(8.0, 5.0, 4.5, 8.5, w), L(8.0, 5.0, 11.5, 8.5, w), L(16.0, 5.0, 16.0, 19.0, w), L(16.0, 19.0, 12.5, 15.5, w), L(16.0, 19.0, 19.5, 15.5, w)]),
        "bt" => v.extend([L(7.0, 8.0, 17.0, 16.0, w), L(17.0, 16.0, 12.0, 20.5, w), L(12.0, 20.5, 12.0, 3.5, w), L(12.0, 3.5, 17.0, 8.0, w), L(17.0, 8.0, 7.0, 16.0, w)]),
        "bolt" => v.push(T(BOLT)),
        "moon" => v.extend([D(12.0, 12.0, 9.0), X(17.0, 8.5, 7.6)]),
        "sun" => {
            v.push(D(12.0, 12.0, 4.4));
            for k in 0..8 {
                let a = (k as f32) * 45.0_f32.to_radians();
                v.push(L(12.0 + 7.4 * a.cos(), 12.0 + 7.4 * a.sin(), 12.0 + 10.2 * a.cos(), 12.0 + 10.2 * a.sin(), w));
            }
        }
        "power" => v.extend([A(12.0, 13.0, 8.0, -58.0, 238.0, w), L(12.0, 3.0, 12.0, 12.0, w)]),
        "gear" => {
            v.extend([C(12.0, 12.0, 6.6, w), C(12.0, 12.0, 2.6, w)]);
            for k in 0..8 {
                let a = (k as f32) * 45.0_f32.to_radians();
                v.push(L(12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin(), 12.0 + 10.2 * a.cos(), 12.0 + 10.2 * a.sin(), 3.6));
            }
        }
        "clock" => v.extend([C(12.0, 12.0, 9.0, w), L(12.0, 12.0, 12.0, 6.5, w), L(12.0, 12.0, 16.0, 14.0, w)]),
        "cal" => v.extend([R(4.0, 5.0, 16.0, 15.0, 3.0, w), L(4.0, 10.0, 20.0, 10.0, w), L(8.0, 3.0, 8.0, 7.0, w), L(16.0, 3.0, 16.0, 7.0, w)]),
        "bell" | "bellx" => {
            v.extend([A(12.0, 10.0, 6.0, 180.0, 360.0, w), T(BELL_BODY), L(3.5, 17.0, 20.5, 17.0, w), D(12.0, 20.5, 1.9)]);
            if name == "bellx" {
                v.push(L(4.0, 4.0, 20.0, 20.0, w));
            }
        }
        "cpu" => {
            v.push(R(6.5, 6.5, 11.0, 11.0, 1.8, w));
            for x in [9.5, 14.5] {
                v.extend([L(x, 2.5, x, 6.5, w), L(x, 17.5, x, 21.5, w), L(2.5, x, 6.5, x, w), L(17.5, x, 21.5, x, w)]);
            }
        }
        "ram" => {
            v.push(R(2.5, 6.5, 19.0, 9.5, 1.8, w));
            for x in [6.5, 10.5, 14.5, 18.5] {
                v.push(L(x, 16.0, x, 19.5, w));
            }
            v.extend([L(7.0, 11.2, 17.0, 11.2, w)]);
        }
        "temp" => v.extend([R(9.3, 2.5, 5.4, 13.0, 2.7, w), D(12.0, 17.8, 4.4), L(12.0, 8.0, 12.0, 16.5, w)]),
        "gauge" => v.extend([A(12.0, 14.0, 9.0, 180.0, 360.0, w), L(12.0, 14.0, 17.0, 8.5, w), D(12.0, 14.0, 2.0)]),
        "lock" => v.extend([F(5.0, 11.0, 14.0, 10.0, 2.5), A(12.0, 10.5, 5.0, 180.0, 360.0, w), L(7.0, 10.5, 7.0, 11.5, w), L(17.0, 10.5, 17.0, 11.5, w)]),
        "user" => v.extend([D(12.0, 8.0, 4.2), A(12.0, 21.5, 8.5, 180.0, 360.0, w)]),
        "folder" => v.extend([F(3.0, 5.5, 8.0, 3.5, 1.2), R(3.0, 8.0, 18.0, 12.0, 2.2, w)]),
        "music" => v.extend([D(8.0, 18.0, 3.2), D(16.5, 16.0, 3.2), L(11.0, 18.0, 11.0, 4.5, w), L(11.0, 4.5, 19.5, 2.8, 2.8), L(19.5, 2.8, 19.5, 16.0, w)]),
        // чашка: корпус, ручка, блюдце и пар
        "coffee" => v.extend([F(3.5, 9.5, 13.0, 9.5, 3.0), A(16.5, 13.5, 3.6, -85.0, 85.0, w), L(2.5, 21.5, 18.5, 21.5, w), L(7.5, 2.5, 7.5, 6.5, w), L(12.0, 2.5, 12.0, 6.5, w)]),
        "play" => v.push(T(PLAY)),
        "pause" => v.extend([F(6.0, 4.0, 4.0, 16.0, 1.0), F(14.0, 4.0, 4.0, 16.0, 1.0)]),
        "next" => v.extend([T(NEXT), F(17.0, 5.0, 3.0, 14.0, 1.0)]),
        "prev" => v.extend([T(PREV), F(4.0, 5.0, 3.0, 14.0, 1.0)]),
        "check" => v.extend([L(5.0, 12.5, 10.0, 17.5, 2.8), L(10.0, 17.5, 19.5, 7.0, 2.8)]),
        "close" => v.extend([L(6.0, 6.0, 18.0, 18.0, w), L(18.0, 6.0, 6.0, 18.0, w)]),
        "plus" => v.extend([L(12.0, 5.0, 12.0, 19.0, w), L(5.0, 12.0, 19.0, 12.0, w)]),
        "min" => v.push(L(5.0, 12.0, 19.0, 12.0, w)),
        "max" => v.push(R(5.0, 5.0, 14.0, 14.0, 2.0, w)),
        "dots" => v.extend([D(5.0, 12.0, 1.9), D(12.0, 12.0, 1.9), D(19.0, 12.0, 1.9)]),
        "menu" => v.extend([L(4.0, 6.0, 20.0, 6.0, w), L(4.0, 12.0, 20.0, 12.0, w), L(4.0, 18.0, 20.0, 18.0, w)]),
        "search" => v.extend([C(10.5, 10.5, 6.5, w), L(15.5, 15.5, 20.5, 20.5, 2.8)]),
        "camera" => v.extend([R(3.0, 7.0, 18.0, 13.0, 3.0, w), C(12.0, 13.5, 3.6, w), L(8.0, 4.8, 16.0, 4.8, w)]),
        "image" => v.extend([R(3.0, 4.0, 18.0, 16.0, 2.2, w), D(9.0, 10.0, 2.0), L(4.0, 18.0, 10.0, 13.0, w), L(10.0, 13.0, 14.0, 16.5, w), L(14.0, 16.5, 16.0, 14.8, w), L(16.0, 14.8, 20.0, 18.5, w)]),
        "drop" => v.extend([D(12.0, 15.0, 6.5), T(DROP_TIP)]),
        "leaf" => v.extend([T(LEAF), L(4.0, 20.0, 13.0, 11.0, w)]),
        "reboot" => v.extend([A(12.0, 12.0, 8.0, -20.0, 250.0, w), T(REBOOT_HEAD)]),
        "exit" => v.extend([R(3.5, 4.0, 10.0, 16.0, 2.0, w), L(10.0, 12.0, 20.0, 12.0, w), T(ARROW_R)]),
        "grid" => v.extend([F(3.0, 3.0, 8.0, 8.0, 2.0), F(13.0, 3.0, 8.0, 8.0, 2.0), F(3.0, 13.0, 8.0, 8.0, 2.0), F(13.0, 13.0, 8.0, 8.0, 2.0)]),
        "tile" => v.extend([R(3.5, 4.5, 7.5, 15.0, 1.8, w), R(13.0, 4.5, 7.5, 15.0, 1.8, w)]),
        "eye" => v.extend([A(12.0, 19.0, 11.0, -130.0, -50.0, w), A(12.0, 5.0, 11.0, 50.0, 130.0, w), D(12.0, 12.0, 3.2)]),
        _ => return None,
    }
    Some(v)
}

fn seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0) };
    ((px - ax - t * dx).powi(2) + (py - ay - t * dy).powi(2)).sqrt()
}

fn rrect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (qx, qy) = ((px - cx).abs() - (w / 2.0 - r), (py - cy).abs() - (h / 2.0 - r));
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

fn poly(px: f32, py: f32, pts: &[(f32, f32)]) -> f32 {
    let mut d = f32::MAX;
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
        d = d.min(seg(px, py, xi, yi, xj, yj));
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    if inside {
        -d
    } else {
        d
    }
}

/// расстояние со знаком (минус — внутри) от точки до фигуры, в единицах 24×24
fn sdf(p: &P, x: f32, y: f32) -> f32 {
    match *p {
        L(x1, y1, x2, y2, w) => seg(x, y, x1, y1, x2, y2) - w / 2.0,
        A(cx, cy, r, a0, a1, w) => {
            let (dx, dy) = (x - cx, y - cy);
            let span = (a1 - a0).rem_euclid(360.0);
            let ang = dy.atan2(dx).to_degrees();
            let d = if (ang - a0).rem_euclid(360.0) <= span {
                ((dx * dx + dy * dy).sqrt() - r).abs()
            } else {
                let e = |a: f32| ((x - (cx + r * a.to_radians().cos())).powi(2) + (y - (cy + r * a.to_radians().sin())).powi(2)).sqrt();
                e(a0).min(e(a1))
            };
            d - w / 2.0
        }
        C(cx, cy, r, w) => (((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r).abs() - w / 2.0,
        D(cx, cy, r) => ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r,
        R(rx, ry, rw, rh, r, w) => rrect(x, y, rx, ry, rw, rh, r).abs() - w / 2.0,
        F(rx, ry, rw, rh, r) => rrect(x, y, rx, ry, rw, rh, r),
        T(pts) => poly(x, y, pts),
        X(..) => f32::MAX,
    }
}

const ZOOM: f32 = 1.12;

/// покрытие значка 0..255 в квадрате size×size
pub fn coverage(name: &str, size: i32) -> Option<Vec<u8>> {
    let shapes = shapes(name)?;
    let k = 24.0 / size as f32; // пикселей → единиц; сглаживание — по краю шириной в пиксель
    let mut out = vec![0u8; (size * size) as usize];
    for j in 0..size {
        for i in 0..size {
            // значок чуть крупнее квадрата 24×24: в квадрате 14 px (бар) иначе он мельче пиксельного
            let (x, y) = (12.0 + ((i as f32 + 0.5) * k - 12.0) / ZOOM, 12.0 + ((j as f32 + 0.5) * k - 12.0) / ZOOM);
            let mut d = f32::MAX;
            for p in &shapes {
                match p {
                    X(..) => {}
                    _ => d = d.min(sdf(p, x, y)),
                }
            }
            for p in &shapes {
                if let X(cx, cy, r) = *p {
                    d = d.max(-(((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r));
                }
            }
            out[(j * size + i) as usize] = ((0.5 - d * ZOOM / k).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    Some(out)
}

/// покрытие произвольной фигуры в пикселях (батарейка и подобное): сглаживание по краю в пиксель
pub fn rect_cov(x: f32, y: f32, w: f32, h: f32, r: f32, stroke: f32, px: f32, py: f32) -> f32 {
    let d = rrect(px, py, x, y, w, h, r);
    let d = if stroke > 0.0 { d.abs() - stroke / 2.0 } else { d };
    (0.5 - d).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// у каждого значка есть что рисовать, и он целиком влезает в свой квадрат
    #[test]
    fn every_shape_draws_something() {
        for n in ["speaker", "mute", "mic", "wifi", "net", "bt", "bolt", "moon", "sun", "power", "gear", "clock", "cal", "bell", "bellx", "cpu", "ram", "temp", "gauge", "lock", "user", "folder", "music", "play", "pause", "next", "prev", "check", "close", "plus", "min", "max", "dots", "menu", "search", "camera", "image", "drop", "leaf", "reboot", "exit", "eye", "grid", "tile", "coffee"] {
            let c = coverage(n, 14).unwrap_or_else(|| panic!("{n}"));
            let lit = c.iter().filter(|v| **v > 0).count();
            assert!(lit > 8, "{n}: {lit}");
            // кромка квадрата пуста: значок не упирается в границу и не обрезается
            let edge: u32 = (0..14).map(|i| c[i] as u32 + c[13 * 14 + i] as u32 + c[i * 14] as u32 + c[i * 14 + 13] as u32).sum();
            assert!(edge < 14 * 4 * 90, "{n}: touches the border ({edge})");
        }
        assert!(coverage("no-such-icon", 14).is_none());
    }

    /// край сглажен: есть и полные, и частичные пиксели
    #[test]
    fn edges_are_antialiased() {
        let c = coverage("clock", 28).unwrap();
        assert!(c.iter().any(|v| *v == 255));
        assert!(c.iter().any(|v| *v > 0 && *v < 255));
    }

    /// выем луны: правая верхняя часть круга убрана
    #[test]
    fn the_moon_has_a_bite() {
        let c = coverage("moon", 24).unwrap();
        assert!(c[12 * 24 + 6] > 200, "left of the moon is lit");
        assert!(c[8 * 24 + 18] < 50, "the bite is empty");
    }

    /// `VICON_SHEET=/tmp/sheet.pgm cargo test -p nemo-px dump_sheet -- --ignored`: все значки одним
    /// листом (серый PGM, 28 px на значок), чтобы посмотреть глазами
    #[test]
    #[ignore]
    fn dump_sheet() {
        let Ok(path) = std::env::var("VICON_SHEET") else { return };
        let names = ["speaker", "mute", "mic", "wifi", "net", "bt", "bolt", "moon", "sun", "power", "gear", "clock", "cal", "bell", "bellx", "cpu", "ram", "temp", "gauge", "lock", "user", "folder", "music", "play", "pause", "next", "prev", "check", "close", "plus", "min", "max", "dots", "menu", "search", "camera", "image", "drop", "leaf", "reboot", "exit", "eye", "grid", "tile", "coffee"];
        let (cols, sz) = (11usize, 28usize);
        let rows = names.len().div_ceil(cols);
        let mut img = vec![0u8; cols * sz * rows * sz];
        for (n, name) in names.iter().enumerate() {
            let c = coverage(name, sz as i32).unwrap();
            let (ox, oy) = ((n % cols) * sz, (n / cols) * sz);
            for j in 0..sz {
                for i in 0..sz {
                    img[(oy + j) * cols * sz + ox + i] = c[j * sz + i];
                }
            }
        }
        let mut out = format!("P5\n{} {}\n255\n", cols * sz, rows * sz).into_bytes();
        out.extend(img);
        std::fs::write(path, out).unwrap();
    }
}
