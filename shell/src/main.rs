// NEMO // шелл для niri на Rust: бар, лаунчер, буфер, питание, настройки, уведомления,
// трей, всплывашки (календарь, звук, сеть, bluetooth, система), виджеты рабочего стола,
// заголовки окон в режиме «стол», блокировка. Рисует сам, пикселями, в shm — без тулкитов.
//
//   nemo-shell                запустить (так зовёт niri)
//   nemo <команда>            то же, что nemo-shell <команда>: справка — `nemo help` (cmd.rs)

pub use nemo_px::{font, gfx, icons, theme};

mod agents;
mod hw;
mod recording;
mod animtab;
mod app;
mod profile;
mod iconsets;
mod restyle;
mod displays;
mod audio;
mod bar;
mod bartab;
mod bright;
mod bttab;
mod sleep;
mod camera;
mod privacy;
mod caffeine;
mod catalog;
mod startup;
mod locate;
mod scriptlib;
mod bundle;
mod calc;
mod cli;
mod clean;
mod click;
mod clock;
mod cmd;
mod dbus;
mod dirs;
mod doctor;
mod dock;
mod fonttab;
mod gameboost;
mod games;
mod i18n;
mod config;
mod control;
mod curtain;
mod ipc;
mod keys;
mod launcher;
mod lite;
mod lyrics;
mod lock;
mod login;
mod media;
mod menu;
mod msgs;
mod net;
mod niri;
mod niricfg;
mod notif;
mod need;
mod night;
mod pam;
mod panels;
mod input;
mod prefs;
mod proc;
mod render;
mod wayland;
mod plugins;
mod recolor;
mod recolor_apps;
mod diagtab;
mod widgetkind;
mod layouts;
mod widgetpanel;
mod kbdlight;
mod testdrive;
mod distro_logo;
mod recolor_art;
mod recolor_data;
mod recolor_math;
mod recolor_tpl;
mod reply;
mod report;
mod personality;
mod hijri;
mod emoji;
mod island;
mod launcher_entry;
mod cheat;
mod popups;
mod setindex;
mod units;
mod sysupd;
mod tips;
mod power;
mod session;
mod setclick;
mod settabs;
mod skwd;
mod screenmap;
mod settings;
mod shot;
mod shotcfg;
mod sys;
mod tick;
mod titles;
mod ui;
mod stability;
mod update;
mod updtab;
mod upinstall;
mod wall;
mod walls;
mod themetab;
mod weather;
mod welcome;
mod widgets;
mod wizinst;
mod wizpages;

fn main() {
    nemo_px::paths::migrate_legacy();
    // sudo спрашивает пароль для установки модулей из первой настройки (wizinst.rs)
    if wizinst::askpass() {
        return;
    }
    // большие буферы (обои, скриншоты, обложки) — сразу через mmap и обратно системе при
    // освобождении: иначе glibc поднимает порог и держит их в куче нитей после каждого декода
    // (musl так не делает, и настроек этих у него нет)
    #[cfg(target_env = "gnu")]
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 256 << 10);
        libc::mallopt(libc::M_ARENA_MAX, 2);
    }
    let args: Vec<String> = std::env::args().collect();
    // `nemo` (ссылка на этот бинарь) без команды — справка; `nemo-shell` без команды — шелл
    let me = args.first().map(|a| a.rsplit('/').next().unwrap_or(a).to_string()).unwrap_or_default();
    if args.len() == 1 && me != "nemo" && me != "nemo" {
        app::run();
        return;
    }
    std::process::exit(cmd::run(&args[1..], &me))
}
