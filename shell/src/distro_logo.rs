// Знак дистрибутива для fastfetch: настоящий значок системы (LOGO= из os-release: fedora-logo-icon, archlinux-logo…),
// сведённый до крупных пикселей и перекрашенный в палитру шелла — как и знак Nemo, с тем же «пиксельным» видом.
// Ничего не рисуется от руки и не качается: берётся то, что дистрибутив положил в систему; нет значка — знак Nemo.

use crate::recolor_math::{rgb, P};
use image::{ImageFormat, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// имя значка из os-release («LOGO=fedora-logo-icon»)
pub fn logo_name(os_release: &str) -> Option<String> {
    let v = os_release.lines().find_map(|l| l.strip_prefix("LOGO="))?;
    let v = v.trim().trim_matches('"').trim_matches('\'');
    (!v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))).then(|| v.to_string())
}

/// файл значка: сначала векторный, иначе самый крупный растровый, в темах значков и в pixmaps
pub fn find_icon(name: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    let mut consider = |score: u32, p: PathBuf| {
        if p.is_file() && best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, p));
        }
    };
    for r in roots {
        consider(1, r.join("pixmaps").join(format!("{name}.png")));
        consider(u32::MAX, r.join("pixmaps").join(format!("{name}.svg")));
        for theme in ["hicolor", "Adwaita", "breeze", "Papirus"] {
            let t = r.join("icons").join(theme);
            for cat in ["apps", "places"] {
                consider(u32::MAX - 1, t.join("scalable").join(cat).join(format!("{name}.svg")));
                if let Ok(rd) = std::fs::read_dir(&t) {
                    for e in rd.flatten() {
                        let size: u32 = e.file_name().to_string_lossy().split('x').next().and_then(|n| n.parse().ok()).unwrap_or(0);
                        if size > 0 {
                            consider(size + 2, e.path().join(cat).join(format!("{name}.png")));
                        }
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

pub fn load(path: &Path) -> Option<RgbaImage> {
    let data = std::fs::read(path).ok()?;
    if path.extension().is_some_and(|e| e == "svg") {
        let img = nemo_px::icons::svg(&data, 256)?;
        let mut out = RgbaImage::new(img.w as u32, img.h as u32);
        for (i, px) in img.px.iter().enumerate() {
            // из предумноженного ARGB — в обычный RGBA
            let a = px >> 24;
            let un = |c: u32| (c * 255).checked_div(a).map_or(0, |v| v.min(255) as u8);
            out.put_pixel(i as u32 % img.w as u32, i as u32 / img.w as u32, Rgba([un((px >> 16) & 255), un((px >> 8) & 255), un(px & 255), a as u8]));
        }
        Some(out)
    } else {
        image::load_from_memory(&data).ok().map(|i| i.to_rgba8())
    }
}

/// тон клетки: 0 — пусто, 1 — акцент (основная заливка), 2 — светлый (белые части знака), 3 — тёмный (контур, тень)
pub const EMPTY: u8 = 0;
pub const MAIN: u8 = 1;
pub const LIGHT: u8 = 2;
pub const DARK: u8 = 3;

pub struct Cells {
    pub n: i32,
    pub tone: Vec<u8>,
}

/// картинка → n×n клеток: обрезать пустые поля, вписать в квадрат, усреднить по площади клетки, поделить на тона
pub fn cells(img: &RgbaImage, n: i32) -> Cells {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, -1, -1);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 40 {
            x0 = x0.min(x as i32);
            y0 = y0.min(y as i32);
            x1 = x1.max(x as i32);
            y1 = y1.max(y as i32);
        }
    }
    if x1 < x0 {
        return Cells { n, tone: vec![EMPTY; (n * n) as usize] };
    }
    let (bw, bh) = (x1 - x0 + 1, y1 - y0 + 1);
    let side = bw.max(bh);
    let (ox, oy) = (x0 - (side - bw) / 2, y0 - (side - bh) / 2);
    let mut tone = vec![EMPTY; (n * n) as usize];
    for cy in 0..n {
        for cx in 0..n {
            let (sx0, sx1) = (ox + cx * side / n, ox + (cx + 1) * side / n);
            let (sy0, sy1) = (oy + cy * side / n, oy + (cy + 1) * side / n);
            let (mut a_sum, mut lum_sum, mut cnt) = (0f32, 0f32, 0f32);
            for y in sy0.max(0)..sy1.min(h).max(sy0.max(0) + 1) {
                for x in sx0.max(0)..sx1.min(w).max(sx0.max(0) + 1) {
                    if x >= w || y >= h {
                        continue;
                    }
                    let p = img.get_pixel(x as u32, y as u32);
                    let a = p[3] as f32 / 255.0;
                    a_sum += a;
                    lum_sum += a * (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0;
                    cnt += 1.0;
                }
            }
            if cnt == 0.0 || a_sum / cnt < 0.5 {
                continue;
            }
            let lum = lum_sum / a_sum;
            tone[(cy * n + cx) as usize] = if lum > 0.72 { LIGHT } else if lum < 0.14 { DARK } else { MAIN };
        }
    }
    Cells { n, tone }
}

fn colour(p: &P, tone: u8) -> (u8, u8, u8) {
    let (r, g, b) = rgb(p.g(match tone {
        LIGHT => "bright",
        DARK => "border",
        _ => "accent",
    }));
    (r as u8, g as u8, b as u8)
}

/// крупными пикселями: каждая клетка — квадрат scale×scale, фон прозрачный (для fastfetch в kitty)
pub fn png(c: &Cells, scale: u32, p: &P) -> Vec<u8> {
    let side = c.n as u32 * scale;
    let mut img = RgbaImage::new(side, side);
    for (i, t) in c.tone.iter().enumerate() {
        if *t == EMPTY {
            continue;
        }
        let (r, g, b) = colour(p, *t);
        let (cx, cy) = (i as u32 % c.n as u32, i as u32 / c.n as u32);
        for dy in 0..scale {
            for dx in 0..scale {
                img.put_pixel(cx * scale + dx, cy * scale + dy, Rgba([r, g, b, 255]));
            }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    let _ = img.write_to(&mut out, ImageFormat::Png);
    out.into_inner()
}

/// полублоками (▀ ▄ █): две клетки по высоте в одной строке, цвета — тона палитры; для остальных терминалов
pub fn text(c: &Cells, p: &P) -> String {
    let at = |x: i32, y: i32| if y < c.n { c.tone[(y * c.n + x) as usize] } else { EMPTY };
    let fg = |t: u8| {
        let (r, g, b) = colour(p, t);
        format!("\u{1b}[38;2;{r};{g};{b}m")
    };
    let bg = |t: u8| {
        let (r, g, b) = colour(p, t);
        format!("\u{1b}[48;2;{r};{g};{b}m")
    };
    let mut rows = vec![];
    for y in (0..c.n).step_by(2) {
        let mut line = String::new();
        for x in 0..c.n {
            let (up, dn) = (at(x, y), at(x, y + 1));
            line += &match (up, dn) {
                (EMPTY, EMPTY) => "\u{1b}[0m ".to_string(),
                (u, EMPTY) => format!("\u{1b}[0m{}▀", fg(u)),
                (EMPTY, d) => format!("\u{1b}[0m{}▄", fg(d)),
                (u, d) if u == d => format!("\u{1b}[0m{}█", fg(u)),
                (u, d) => format!("\u{1b}[0m{}{}▀", fg(u), bg(d)),
            };
        }
        rows.push(format!("{line}\u{1b}[0m"));
    }
    rows.join("\n") + "\n"
}

/// значок этой системы: найти и свести; None — значка нет (тогда знак Nemo)
pub fn current(n: i32) -> Option<Cells> {
    let osr = std::fs::read_to_string("/etc/os-release").or_else(|_| std::fs::read_to_string("/usr/lib/os-release")).ok()?;
    let name = logo_name(&osr)?;
    let mut roots: Vec<PathBuf> = nemo_px::paths::data_dirs();
    roots.insert(0, nemo_px::paths::data_dir());
    let img = load(&find_icon(&name, &roots)?)?;
    let c = cells(&img, n);
    // значок, от которого осталась пара клеток, — не значок
    (c.tone.iter().filter(|t| **t != EMPTY).count() >= (n * n / 8) as usize).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_name_comes_from_os_release() {
        assert_eq!(logo_name("ID=fedora\nLOGO=fedora-logo-icon\n"), Some("fedora-logo-icon".into()));
        assert_eq!(logo_name("LOGO=\"archlinux-logo\""), Some("archlinux-logo".into()));
        assert_eq!(logo_name("ID=x"), None);
        assert_eq!(logo_name("LOGO=../../etc/passwd"), None, "a name, never a path");
    }

    fn disc() -> RgbaImage {
        // синий круг с белым квадратом посередине, прозрачные углы
        let mut img = RgbaImage::new(64, 64);
        for y in 0..64i32 {
            for x in 0..64i32 {
                let (dx, dy) = (x - 32, y - 32);
                if dx * dx + dy * dy <= 30 * 30 {
                    let white = dx.abs() < 10 && dy.abs() < 10;
                    img.put_pixel(x as u32, y as u32, if white { Rgba([255, 255, 255, 255]) } else { Rgba([60, 110, 180, 255]) });
                }
            }
        }
        img
    }

    #[test]
    fn a_two_tone_disc_becomes_main_ring_light_centre_and_empty_corners() {
        let c = cells(&disc(), 16);
        let at = |x: i32, y: i32| c.tone[(y * 16 + x) as usize];
        assert_eq!(at(0, 0), EMPTY, "corner");
        assert_eq!(at(8, 8), LIGHT, "centre");
        assert_eq!(at(8, 2), MAIN, "ring");
    }

    #[test]
    fn an_empty_picture_gives_no_cells() {
        assert!(cells(&RgbaImage::new(8, 8), 8).tone.iter().all(|t| *t == EMPTY));
    }

    #[test]
    fn the_icon_is_found_by_size_and_a_vector_wins() {
        let d = nemo_px::paths::private_tmp("distro-logo").unwrap();
        for s in ["48x48", "256x256"] {
            let dir = d.join(format!("icons/hicolor/{s}/apps"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("x-logo.png"), "p").unwrap();
        }
        let got = find_icon("x-logo", std::slice::from_ref(&d)).unwrap();
        assert!(got.to_string_lossy().contains("256x256"), "{got:?}");
        let sv = d.join("icons/hicolor/scalable/apps");
        std::fs::create_dir_all(&sv).unwrap();
        std::fs::write(sv.join("x-logo.svg"), "s").unwrap();
        assert!(find_icon("x-logo", std::slice::from_ref(&d)).unwrap().to_string_lossy().ends_with(".svg"));
        assert_eq!(find_icon("nothing-here", std::slice::from_ref(&d)), None);
        let _ = std::fs::remove_dir_all(d);
    }
}
