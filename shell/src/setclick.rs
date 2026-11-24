// Настройки: что делает щелчок (Act::S…) — меняет Settings и применяет (settings_changed);
// вкладкам со своими файлами — их обработчики (обои, клавиши, обновления).

use crate::app::{App, Msg, SKind};
use crate::config::Widget;
use crate::settings::cycle;
use crate::theme::{parse_hex, PRESETS};
use crate::ui::Act;
use crate::widgets;

impl App {
    /// клики внутри настроек; true — обработано
    pub fn settings_click(&mut self, a: &Act) -> bool {
        if self.welcome_click(a) || self.anim_click(a) {
            return true;
        }
        let s = &mut self.st.set;
        match a {
            Act::STab(id) => {
                self.st.a11y_focus = None;
                self.st.stab = id;
                self.st.search_field.set("");
                self.st.field_focus = None;
                self.keys_capture(None);
                if *id == "screens" {
                    self.disp_open();
                }
                self.tab_probe();
            }
            Act::SPreset(i) => {
                s.palette = PRESETS[*i].id.into();
                s.accent.clear();
                // цвет выбран руками: обои его больше не перекрашивают
                s.wall_accent = false;
                self.st.hex.set(&format!("#{:06x}", PRESETS[*i].accent));
                self.settings_changed();
            }
            Act::SNum(k, d) => {
                s.step_num(k, *d);
                self.settings_changed();
                // отступы уведомлений и OSD: поверхность создаётся заново на новом месте
                match *k {
                    "notif_offset" => self.renew_notif(SKind::Notifs),
                    "osd_offset" => self.renew_notif(SKind::Osd),
                    _ => {}
                }
            }
            Act::STplToggle(i) => {
                if let Some(f) = self.st.restyle.tpls.get(*i).map(|t| t.file.clone()) {
                    match s.templates_off.iter().position(|x| *x == f) {
                        Some(k) => drop(s.templates_off.remove(k)),
                        None => s.templates_off.push(f),
                    }
                    // recolor читает настройки с диска
                    self.save_now();
                    self.templates_run(None);
                }
            }
            Act::SCtlTile(k) => {
                let l = &mut self.st.set.control_tiles;
                if let Some(i) = l.iter().position(|x| x == k) {
                    l.remove(i);
                } else if l.len() < crate::control::MAX {
                    l.push(k.to_string());
                }
                self.settings_changed();
            }
            Act::STplAdd(i) => {
                if crate::catalog::add(*i) {
                    self.templates_run(None);
                }
            }
            Act::STplForce(i) => {
                if let Some(f) = self.st.restyle.tpls.get(*i).map(|t| t.file.clone()) {
                    self.templates_run(Some(f));
                }
            }
            Act::SThemeApp(id) => {
                let on = match s.theme_apps.iter().position(|a| a == id) {
                    Some(i) => {
                        s.theme_apps.remove(i);
                        false
                    }
                    None => {
                        s.theme_apps.push(id.to_string());
                        true
                    }
                };
                // kitty включили руками: наш файл и строка include (выключение её уберёт recolor)
                if on && *id == "kitty" {
                    crate::prefs::kitty_attach();
                }
                self.settings_changed();
                self.theme_rerun();
                return true;
            }
            // части nemo-agents: их файл настроек, не настройки шелла
            Act::SBool(k) if matches!(*k, "agent_polkit" | "agent_cast" | "automount" | "automount_open") => {
                crate::agents::AgentsCfg::flip(k);
                crate::agents::write_portals(&self.st.set);
                self.dirty(&SKind::Modal);
            }
            Act::SBool(k) if k.starts_with("shot_") => {
                // настройки скриншотов — файл nemo-shot, а не настройки шелла
                self.st.shot_cfg.fresh();
                let c = &mut self.st.shot_cfg;
                match *k {
                    "shot_copy" => c.copy = !c.copy,
                    "shot_save" => c.save = !c.save,
                    "shot_loupe" => c.loupe = !c.loupe,
                    "shot_notify" => c.notify = !c.notify,
                    _ => {}
                }
                c.save();
                self.dirty(&SKind::Modal);
            }
            Act::SBool(k) => {
                match *k {
                    "animations" => s.animations = !s.animations,
                    "high_contrast" => s.high_contrast = !s.high_contrast,
                    "reduce_motion" => s.reduce_motion = !s.reduce_motion,
                    "lite" => s.lite = !s.lite,
                    "anim_close" => s.anim_close = !s.anim_close,
                    "scanlines" => s.scanlines = !s.scanlines,
                    "lock_sleep" => s.lock_sleep = !s.lock_sleep,
                    "osd_bright" => s.osd_bright = !s.osd_bright,
                    "icon_pixel" => s.icon_pixel = !s.icon_pixel,
                    "icon_tint" => s.icon_tint = !s.icon_tint,
                    "stacking" => {
                        s.stacking = !s.stacking;
                        let on = s.stacking;
                        self.st.niri.set_stacking(on);
                        self.sync_titles();
                    }
                    "titlebars" => {
                        s.titlebars = !s.titlebars;
                        self.sync_titles();
                    }
                    "wall_kanji" => s.wall_kanji = !s.wall_kanji,
                    "theme_gtk4" => {
                        s.theme_gtk4 = !s.theme_gtk4;
                        self.theme_rerun();
                        self.dirty(&SKind::Modal);
                        return true;
                    }
                    "wall_skwd" | "wall_skwd_colors" => {
                        if *k == "wall_skwd" {
                            s.wall_skwd = !s.wall_skwd;
                        } else {
                            s.wall_skwd_colors = !s.wall_skwd_colors;
                        }
                        self.settings_changed();
                        // сразу, не ждать тика: обои и палитра skwd-wall
                        self.skwd_tick();
                        self.skwd_colors();
                        self.dirty(&SKind::Modal);
                        return true;
                    }
                    "bar_float" => s.bar_float = !s.bar_float,
                    "bar_split" => s.bar_split = !s.bar_split,
                    "widget_contrast" => s.widget_contrast = !s.widget_contrast,
                    "blur_shell" => s.blur_shell = !s.blur_shell,
                    "blur_term" => s.blur_term = !s.blur_term,
                    "blur_real" => s.blur_xray = !s.blur_xray,
                    "clock_seconds" => s.clock_seconds = !s.clock_seconds,
                    "tray_pixel" => s.tray_pixel = !s.tray_pixel,
                    "bat_hide_full" => s.bat_hide_full = !s.bat_hide_full,
                    "privacy_mic" => s.privacy_mic = !s.privacy_mic,
                    "privacy_cam" => s.privacy_cam = !s.privacy_cam,
                    "privacy_screen" => s.privacy_screen = !s.privacy_screen,
                    "clock_date" => s.clock_date = !s.clock_date,
                    "hijri" => s.hijri = !s.hijri,
                    "week_numbers" => s.week_numbers = !s.week_numbers,
                    "arabic_digits" => s.arabic_digits = !s.arabic_digits,
                    "dnd" => s.dnd = !s.dnd,
                    "island" => s.island = !s.island,
                    "caffeine_auto" => s.caffeine_auto = !s.caffeine_auto,
                    "float_media" => s.float_media = !s.float_media,
                    "dock" => s.dock = !s.dock,
                    "dock_bounce" => s.dock_bounce = !s.dock_bounce,
                    "dock_running" => s.dock_running = !s.dock_running,
                    "dock_labels" => s.dock_labels = !s.dock_labels,
                    "dock_badges" => s.dock_badges = !s.dock_badges,
                    "dock_launcher" => s.dock_launcher = !s.dock_launcher,
                    "dock_trash" => s.dock_trash = !s.dock_trash,
                    "float_pip" => s.float_pip = !s.float_pip,
                    "game_fullscreen" => s.game_fullscreen = !s.game_fullscreen,
                    "game_vrr" => s.game_vrr = !s.game_vrr,
                    "game_tearing" => s.game_tearing = !s.game_tearing,
                    "game_cache" => s.game_cache = !s.game_cache,
                    "game_dnd" => s.game_dnd = !s.game_dnd,
                    "game_profile" => s.game_profile = !s.game_profile,
                    "game_ntsync" => s.game_ntsync = !s.game_ntsync,
                    "game_gamemode" => s.game_gamemode = !s.game_gamemode,
                    "night" => s.night = !s.night,
                    "night_always" => s.night_always = !s.night_always,
                    "clock12" => s.clock12 = !s.clock12,
                    "niri_look" => s.niri_look = !s.niri_look,
                    "niri_keys" => s.niri_keys = !s.niri_keys,
                    "niri_rules" => s.niri_rules = !s.niri_rules,
                    "niri_colors" => s.niri_colors = !s.niri_colors,
                    "niri_screens" => s.niri_screens = !s.niri_screens,
                    "session_restore" => s.session_restore = !s.session_restore,
                    "update_check" => s.update_check = !s.update_check,
                    "terminal_fish" => {
                        // recolor читает настройки с диска: сохранить, потом переписать kitty/nemo-shell.conf
                        s.terminal_fish = !s.terminal_fish;
                        self.settings_changed();
                        self.save_now();
                        std::thread::spawn(|| {
                            crate::proc::recolor_run(&["--fonts".as_ref()]);
                        });
                        self.dirty(&SKind::Modal);
                        return true;
                    }
                    "fetch_full" => {
                        // recolor читает настройки с диска: сохранить, потом перерисовать fastfetch
                        s.fetch_full = !s.fetch_full;
                        self.settings_changed();
                        self.save_now();
                        std::thread::spawn(|| {
                            crate::proc::recolor_run(&["--fetch".as_ref()]);
                        });
                        self.dirty(&SKind::Modal);
                        return true;
                    }
                    k if crate::login::toggle(s, k) => {}
                    k if crate::agents::toggle(s, k) => {
                        // тексты из сети включили: «не нашлось» для текущего трека — искать заново
                        if k == "lyrics_net" {
                            self.st.lyrics = Default::default();
                            self.lyrics_want();
                        }
                    }
                    "browser_menus" | "browser_font" => {
                        if *k == "browser_menus" {
                            s.browser_menus = !s.browser_menus;
                        } else {
                            s.browser_font = !s.browser_font;
                        }
                        self.settings_changed();
                        self.apply_fonts();
                        self.dirty(&SKind::Modal);
                        return true;
                    }
                    _ => {}
                }
                self.settings_changed();
                // «остров» меняет место карточек и громкости: поверхности — заново (якорь на лету не меняется)
                if *k == "island" {
                    self.renew_notif(SKind::Notifs);
                    self.renew_notif(SKind::Osd);
                    self.dirty(&SKind::Bar);
                }
                // превью наборов — в новом виде значков
                if matches!(*k, "icon_pixel" | "icon_tint") && self.st.sets.is_some() {
                    self.st.sets = None;
                    self.sets_load();
                }
            }
            Act::SIconSet(i) => {
                let Some(name) = self.st.sets.as_ref().and_then(|x| x.icons.get(*i)).map(|c| c.name.clone()) else { return true };
                s.icon_theme = name;
                self.icons_apply();
            }
            Act::SCursorSet(i) => {
                let Some(name) = self.st.sets.as_ref().and_then(|x| x.cursors.get(*i)).map(|c| c.name.clone()) else { return true };
                s.cursor = name;
                self.settings_changed();
                self.apply_cursor();
            }
            Act::SCycle(k) if k.starts_with("font_") && *k != "font_size" => {
                let k: &str = k;
                return self.font_cycle(k);
            }
            Act::SPlace(k, v) => {
                match *k {
                    "bar_pos" => s.bar_pos = v.to_string(),
                    "notif_pos" => s.notif_pos = v.to_string(),
                    "osd_pos" => s.osd_pos = v.to_string(),
                    _ => return true,
                }
                self.settings_changed();
                match *k {
                    "notif_pos" => {
                        self.renew_notif(SKind::Notifs);
                        self.notif_preview();
                    }
                    "osd_pos" => {
                        self.renew_notif(SKind::Osd);
                        self.show_osd("volume");
                    }
                    _ => {}
                }
            }
            Act::SCycle(k) => match *k {
                "cpu_style" => {
                    s.cpu_style = cycle(&[("gauge", ""), ("graph", ""), ("number", "")], &s.cpu_style).into();
                    self.settings_changed();
                }
                "notif_pos" => {
                    s.notif_pos = cycle(crate::notif::POS, &s.notif_pos).into();
                    self.settings_changed();
                    self.renew_notif(SKind::Notifs);
                }
                "osd_pos" => {
                    s.osd_pos = cycle(crate::notif::OSD_POS, &s.osd_pos).into();
                    self.settings_changed();
                    self.renew_notif(SKind::Osd);
                }
                "record_audio" => {
                    self.st.shot_cfg.fresh();
                    let c = &mut self.st.shot_cfg;
                    c.record_audio = match c.record_audio.as_str() {
                        "off" => "mic",
                        "mic" => "system",
                        _ => "off",
                    }
                    .into();
                    c.save();
                    self.dirty(&SKind::Modal);
                }
                "shot_confirm" => {
                    self.st.shot_cfg.fresh();
                    let c = &mut self.st.shot_cfg;
                    c.confirm = if c.confirm == "instant" { "space" } else { "instant" }.into();
                    c.save();
                    self.dirty(&SKind::Modal);
                }
                "media_view" => self.media_view_next(),
                "media_viz" => {
                    let mut l = crate::widgets::STYLES.to_vec();
                    l.push(("off", "выкл"));
                    s.media_viz = cycle(&l, &s.media_viz).into();
                    self.media_sync();
                }
                "tray_mode" => {
                    s.tray_mode = if s.tray_mode == "arrow" { "icons" } else { "arrow" }.into();
                    self.settings_changed();
                }
                "temp_style" => {
                    s.temp_style = if s.temp_style == "compact" { "full" } else { "compact" }.into();
                    self.settings_changed();
                }
                "color_page_pal" | "color_page_themes" | "color_page_apps" => {
                    self.st.color_page = match *k {
                        "color_page_themes" => 1,
                        "color_page_apps" => 2,
                        _ => 0,
                    };
                    if self.st.color_page == 1 {
                        self.theme_do("list", 0);
                    }
                    if self.st.color_page == 2 {
                        self.st.restyle = crate::restyle::load();
                    }
                    self.dirty(&SKind::Modal);
                }
                "theme_rerun" => self.theme_rerun(),
                "templates_open" => {
                    let d = crate::config::config_dir().join("nemo/templates");
                    crate::prefs::templates_example(&d);
                    App::open_dir(d.display().to_string());
                }
                "theme_save" => self.theme_do("save", 0),
                "bar_page_mods" | "bar_page_look" => {
                    self.st.bar_ed.look = *k == "bar_page_look";
                    self.dirty(&SKind::Modal);
                }
                "bar_center_align" => {
                    s.bar_center_align = if s.bar_center_align == "after-left" { "center" } else { "after-left" }.into();
                    self.settings_changed();
                }
                "bar_pos" => {
                    s.bar_pos = cycle(crate::bar::EDGES, &s.bar_pos).into();
                    self.settings_changed();
                }
                "bar_sep" => {
                    s.bar_sep = cycle(&[("dots", ""), ("line", ""), ("blocks", ""), ("gap", "")], &s.bar_sep).into();
                    self.settings_changed();
                }
                "bar_sep_color" => {
                    s.bar_sep_color = cycle(&[("border", ""), ("accent", ""), ("dim", "")], &s.bar_sep_color).into();
                    self.settings_changed();
                }
                "reveal" => {
                    s.reveal = cycle(&[("dither", ""), ("scan", ""), ("noise", ""), ("wipe", ""), ("blocks", ""), ("fade", ""), ("none", "")], &s.reveal).into();
                    self.settings_changed();
                }
                // стиль тянет за собой то, что без него выглядит чужим: проявление, значки, матовость
                "preset" => {
                    let plain = s.preset != "plain";
                    s.preset = if plain { "plain" } else { "nemo" }.into();
                    s.reveal = if plain { "fade" } else { "dither" }.into();
                    s.icon_pixel = !plain;
                    s.glass = if plain { 1.0 } else { 0.55 };
                    self.settings_changed();
                }
                "corners" => {
                    s.corners = cycle(&[("", ""), ("notch", ""), ("round", ""), ("square", "")], &s.corners).into();
                    self.settings_changed();
                }
                "shadow" => {
                    s.shadow = cycle(&[("", ""), ("hard", ""), ("soft", ""), ("none", "")], &s.shadow).into();
                    self.settings_changed();
                }
                "radius" => {
                    s.radius = match s.radius {
                        -1 => 0,
                        0 => 4,
                        4 => 6,
                        6 => 8,
                        8 => 12,
                        12 => 16,
                        _ => -1,
                    };
                    self.settings_changed();
                }
                "open_setup" => self.welcome_open("modules"),
                "open_welcome" => self.welcome_open("hello"),
                "widget_gallery" => {
                    self.st.widget_gallery = !self.st.widget_gallery;
                    self.st.widget_panels = false;
                    self.st.widget_layouts = false;
                    self.st.widget_scripts = false;
                    self.dirty(&SKind::Modal);
                }
                "widget_panels" => {
                    self.st.widget_panels = !self.st.widget_panels;
                    self.st.widget_gallery = false;
                    self.st.widget_layouts = false;
                    self.st.widget_scripts = false;
                    self.dirty(&SKind::Modal);
                }
                "widget_layouts" => {
                    self.st.widget_layouts = !self.st.widget_layouts;
                    self.st.widget_scripts = false;
                    self.st.widget_panels = false;
                    self.st.widget_gallery = false;
                    self.dirty(&SKind::Modal);
                }
                "widget_scripts" => {
                    self.st.widget_scripts = !self.st.widget_scripts;
                    self.st.widget_layouts = false;
                    self.st.widget_panels = false;
                    self.st.widget_gallery = false;
                    self.dirty(&SKind::Modal);
                }
                "upd_page_desktop" | "upd_page_software" => {
                    self.st.upd_page = if *k == "upd_page_software" { 1 } else { 0 };
                    if self.st.upd_page == 1 {
                        let tx = self.tx.clone();
                        crate::sysupd::start(&mut self.st, tx, false);
                    }
                    self.dirty(&SKind::Modal);
                }
                "sysupd_check" => {
                    let tx = self.tx.clone();
                    crate::sysupd::start(&mut self.st, tx, true);
                    self.dirty(&SKind::Modal);
                }
                "sysupd_run" => {
                    if let Some(cmd) = self.st.sys_upd.fam.as_ref().and_then(|f| crate::sysupd::update_command(f.0)) {
                        // окно не закрывается само: итог виден, Enter закрывает; sudo спросит пароль там
                        let line = format!("{cmd}; printf '\\n[Enter]'; read _");
                        App::spawn(&crate::launcher::in_term(vec!["sh".into(), "-c".into(), line]));
                        // после обновления список устарел: следующая проверка — заново
                        self.st.sys_upd.found = None;
                    }
                }
                "diag_refresh" => self.st.diag = Some(crate::diagtab::collect()),
                "diag_open_cfg" => App::spawn(&["xdg-open".to_string(), nemo_px::paths::nemo_dir().to_string_lossy().to_string()]),
                "diag_open_state" => App::spawn(&["xdg-open".to_string(), nemo_px::paths::state_dir().to_string_lossy().to_string()]),
                "diag_report" => {
                    // архив для issue в домашней папке; команда та же, что в терминале: nemo report
                    let tm = nemo_px::time::local_now();
                    let file = nemo_px::paths::home().join(format!("nemo-report-{:04}{:02}{:02}-{:02}{:02}.tar.gz", tm.year, tm.mon, tm.day, tm.hour, tm.min));
                    App::spawn(&[nemo_px::paths::self_exe().to_string_lossy().to_string(), "report".into(), "-o".into(), file.to_string_lossy().to_string()]);
                    self.toast_note("состояние", &crate::tf!("отчёт: {}", "report: {}", "relatório: {}", nemo_px::paths::pretty(&file)));
                }
                "sysupd_run_safe" => {
                    // решает сам скрипт: что поставит, что отложит и почему; терминал остаётся открытым, пока читаешь итог
                    let line = "nemo-fedora-update --when-safe; printf '\\n[Enter]'; read _".to_string();
                    App::spawn(&crate::launcher::in_term(vec!["sh".into(), "-c".into(), line]));
                    self.st.sys_upd.found = None;
                }
                "safe_timer" => {
                    // таймер — юнит systemd пользователя; включает и выключает сам скрипт, шелл только смотрит, есть ли файл
                    let arg = if crate::sysupd::safe_timer_on() { "off" } else { "on" };
                    App::spawn(&["nemo-fedora-update".to_string(), "--timer".to_string(), arg.to_string()]);
                    self.toast_note("обновления", if arg == "on" { "раз в день: ставится то, что безопасно" } else { "таймер выключен" });
                    self.dirty(&SKind::Modal);
                }
                "sysupd_run_all" => {
                    // всё, включая удержанное (ядро, драйверы, основа): человек уверен; терминал переспросит, снимок делается
                    let line = "nemo-fedora-update --apply --include-held; printf '\\n[Enter]'; read _".to_string();
                    App::spawn(&crate::launcher::in_term(vec!["sh".into(), "-c".into(), line]));
                    self.st.sys_upd.found = None;
                }
                "prof_battery" | "prof_ac" => {
                    let opts = [("", ""), ("power-saver", ""), ("balanced", ""), ("performance", "")];
                    if *k == "prof_battery" {
                        s.profile_battery = cycle(&opts, &s.profile_battery).into();
                    } else {
                        s.profile_ac = cycle(&opts, &s.profile_ac).into();
                    }
                    // правило сработает по новой настройке сразу, а не при следующей смене питания
                    self.st.pol_prev = None;
                }
                "caffeine_30" | "caffeine_60" | "caffeine_120" | "caffeine_240" | "caffeine_inf" => {
                    let m = match *k {
                        "caffeine_30" => 30,
                        "caffeine_60" => 60,
                        "caffeine_120" => 120,
                        "caffeine_240" => 240,
                        _ => 0,
                    };
                    self.caffeine_set(true, m);
                    self.dirty(&SKind::Modal);
                }
                "reset_page" => {
                    let tab = self.st.stab;
                    let armed = self.st.reset_armed.is_some_and(|(t, at)| t == tab && at.elapsed().as_secs() < 5);
                    if !armed {
                        self.st.reset_armed = Some((tab, std::time::Instant::now()));
                    } else {
                        self.st.reset_armed = None;
                        let n = self.st.set.reset_page(tab);
                        self.settings_changed();
                        self.save_now();
                        // шрифты программ и курсор перекрашивает recolor: только если ключи страницы их касаются
                        if tab == "fonts" || tab == "look" {
                            self.apply_fonts();
                            self.apply_cursor();
                        }
                        self.toast_note("настройки", if n == 0 { "на этой странице всё и так по умолчанию" } else { "страница возвращена к значениям по умолчанию" });
                    }
                    self.dirty(&SKind::Modal);
                }
                "scripts_add" => {
                    for (n, _) in crate::scriptlib::ALL {
                        let _ = crate::scriptlib::add(n);
                    }
                    self.settings_changed();
                    self.dirty(&SKind::Modal);
                }
                "icon_style" => {
                    s.icon_style = cycle(&[("", ""), ("pixel", ""), ("normal", "")], &s.icon_style).into();
                    self.settings_changed();
                }
                "widget_panel" => {
                    s.widget_panel = cycle(&[("glass", ""), ("solid", ""), ("outline", "")], &s.widget_panel).into();
                    self.settings_changed();
                }
                "widget_look" => {
                    s.widget_look = cycle(&[("plain", ""), ("reef", "")], &s.widget_look).into();
                    self.settings_changed();
                }
                "bar_panel" => {
                    let all: Vec<&str> = std::iter::once("").chain(crate::widgetpanel::STYLES.iter().map(|x| x.0)).collect();
                    let i = all.iter().position(|x| *x == s.bar_panel).unwrap_or(0);
                    s.bar_panel = all[(i + 1) % all.len()].to_string();
                    self.settings_changed();
                }
                "kbd_light" => {
                    s.kbd_light = cycle(&[("idle_battery", ""), ("idle", ""), ("keep", "")], &s.kbd_light).into();
                    self.settings_changed();
                }
                "kbd_idle" => {
                    s.kbd_idle_secs = crate::settings::step_in(&[10, 20, 30, 60, 120], s.kbd_idle_secs as i32) as u32;
                    self.settings_changed();
                }
                "personality" => {
                    s.personality = cycle(&[("quiet", ""), ("playful", ""), ("off", "")], &s.personality).into();
                }
                "date_style" => {
                    s.date_style = cycle(&[("dm", ""), ("md", ""), ("iso", "")], &s.date_style).into();
                    self.settings_changed();
                }
                "weekend" => {
                    s.weekend = cycle(&[("satsun", ""), ("frisat", "")], &s.weekend).into();
                }
                "week_start" => {
                    s.week_start = cycle(&[("mon", ""), ("sun", ""), ("sat", "")], &s.week_start).into();
                    self.settings_changed();
                }
                "dock_anim" => {
                    s.dock_anim = cycle(&[("grow", ""), ("wave", ""), ("lift", ""), ("none", "")], &s.dock_anim).into();
                    self.settings_changed();
                }
                "font_size" => {
                    s.font_size = match s.font_size {
                        0 => 16,
                        16 => 24,
                        24 => 32,
                        _ => 0,
                    };
                    self.settings_changed();
                    self.apply_fonts();
                }
                "hex_apply" => self.apply_hex(),
                "theme_dark" | "theme_light" => {
                    s.light = *k == "theme_light";
                    self.settings_changed();
                }
                "lang" => {
                    s.lang = nemo_px::i18n::next(&s.lang).into();
                    self.settings_changed();
                }
                "wall_apply" => {
                    let p = self.st.wall_field.text.clone();
                    if !self.wall_dir_typed(&p) {
                        self.set_wall(&p);
                    }
                }
                "curtain_style" => {
                    s.curtain_style = crate::curtain::next_style(&s.curtain_style).into();
                    self.settings_changed();
                }
                "shot_delay" => {
                    self.st.shot_cfg.fresh();
                    let c = &mut self.st.shot_cfg;
                    c.delay = match c.delay {
                        3 => 5,
                        5 => 10,
                        _ => 3,
                    };
                    c.save();
                    self.dirty(&SKind::Modal);
                }
                "set_add" => self.sets_add(),
                "cursor_size" => {
                    s.cursor_size = match s.cursor_size {
                        24 => 32,
                        32 => 48,
                        48 => 64,
                        _ => 24,
                    };
                    self.settings_changed();
                    self.apply_cursor();
                }
                "dock_side" => {
                    s.dock_side = match s.dock_side.as_str() {
                        "bottom" => "left",
                        "left" => "right",
                        "right" => "top",
                        _ => "bottom",
                    }
                    .into();
                    // край сменился — пересоздать
                    for k in [SKind::Dock, SKind::DockEdge] {
                        for id in self.find(&k) {
                            self.destroy(id);
                        }
                    }
                    self.settings_changed();
                }
                "dock_hide" => {
                    let s = &mut self.st.set;
                    s.dock_hide = match s.dock_hide.as_str() {
                        "smart" => "always",
                        "always" => "never",
                        _ => "smart",
                    }
                    .into();
                    for k in [SKind::Dock, SKind::DockEdge] {
                        for id in self.find(&k) {
                            self.destroy(id);
                        }
                    }
                    self.settings_changed();
                }
                "kb_apply" => {
                    self.st.set.kb_layout = self.st.keys.lay.text.trim().to_string();
                    self.st.keys.lay.clear();
                    self.st.field_focus = None;
                    self.settings_changed();
                }
                "kb_switch" => {
                    let sw = crate::keys::SWITCHES;
                    let i = sw.iter().position(|x| x.0 == self.st.set.kb_switch).map(|i| (i + 1) % sw.len()).unwrap_or(0);
                    self.st.set.kb_switch = sw[i].0.to_string();
                    self.settings_changed();
                }
                "weather_apply" => {
                    // город сменился: старый ответ не показывать, спросить сразу
                    self.st.set.weather_city = self.st.weather_field.text.trim().chars().take(64).collect();
                    self.st.field_focus = None;
                    self.weather_reset();
                }
                "weather_units" => {
                    self.st.set.weather_units = if self.st.set.weather_units == "f" { "c" } else { "f" }.into();
                    self.weather_reset();
                }
                "geo_apply" => {
                    let g = self.st.geo_field.text.trim().to_string();
                    // пусто — по часам; иначе только верные координаты
                    if g.is_empty() || crate::night::geo(&g).is_some() || crate::locate::is_auto(&g) {
                        self.st.set.night_geo = g;
                        self.st.field_focus = None;
                        self.settings_changed();
                    }
                }
                "game_apply" => {
                    let p = self.st.game_field.text.trim().to_string();
                    self.st.set.game_apps = p;
                    self.st.field_focus = None;
                    self.settings_changed();
                }
                "muted_apply" => {
                    self.st.set.notif_muted = self.st.muted_field.text.trim().to_string();
                    self.st.field_focus = None;
                    self.settings_changed();
                }
                "float_apply" => {
                    let p = self.st.float_field.text.trim().to_string();
                    self.st.set.float_apps = p;
                    self.st.field_focus = None;
                    self.settings_changed();
                }
                "cast_apply" => {
                    let p = self.st.cast_field.text.trim().to_string();
                    self.st.set.cast_hide = p;
                    self.st.field_focus = None;
                    self.settings_changed();
                }
                "rec_dir_apply" => {
                    let p = self.st.rec_field.text.trim().to_string();
                    self.st.shot_cfg.fresh();
                    self.st.shot_cfg.record_dir = p;
                    self.st.shot_cfg.save();
                    self.st.field_focus = None;
                    self.dirty(&SKind::Modal);
                }
                "record_fps" => {
                    self.st.shot_cfg.fresh();
                    let c = &mut self.st.shot_cfg;
                    c.record_fps = match c.record_fps {
                        0 => 30,
                        30 => 60,
                        _ => 0,
                    };
                    c.save();
                    self.dirty(&SKind::Modal);
                }
                "shot_dir_apply" => {
                    let p = self.st.shot_field.text.trim().to_string();
                    self.st.shot_cfg.fresh();
                    self.st.shot_cfg.dir = p;
                    self.st.shot_cfg.save();
                    self.st.field_focus = None;
                    self.dirty(&SKind::Modal);
                }
                "do_lock" => {
                    self.close_modal();
                    self.lock_now();
                }
                "reload" => self.ipc("reload"),
                k if self.update_cycle(k) || self.agents_cycle(k) || self.login_cycle(k) => {}
                "niri_connect" | "niri_disconnect" => {
                    // niri validate — отдельный процесс: не на нити Wayland
                    let on = *k == "niri_connect";
                    self.save_now();
                    let tx = self.tx.clone();
                    std::thread::spawn(move || {
                        let r = if on { crate::niricfg::connect(crate::niricfg::Mode::Auto) } else { crate::niricfg::disconnect() };
                        let _ = tx.send(Msg::NiriLink(crate::niricfg::status(), r.unwrap_or_else(|e| e)));
                    });
                }
                "open_config" => {
                    // на диске должно быть то, что сейчас в настройках
                    let j = self.st.set.to_json();
                    let _ = crate::config::write_atomic(&crate::config::settings_path(), &j);
                    self.remember_saved(j);
                    // настройки — слой над окнами: редактор открылся бы под ними (#8)
                    self.close_modal();
                    App::spawn(&["xdg-open".into(), crate::config::settings_path().display().to_string()]);
                }
                _ => {}
            },
            Act::STheme(i) => self.theme_do("apply", *i),
            Act::SThemeDel(i) => self.theme_do("remove", *i),
            Act::SModDel(si, i) => {
                let l = s.side_mut(*si);
                if *i < l.len() {
                    l.remove(*i);
                }
                self.settings_changed();
            }
            Act::SModNew(m) => {
                // клик (не перетаскивание): в привычное место
                let m = m.to_string();
                let m = &m;
                if m == "clock" {
                    s.bar_center.push(m.clone());
                } else if matches!(m.as_str(), "tasks" | "launcher" | "workspaces" | "mode") {
                    s.bar_left.push(m.clone());
                } else {
                    // справа — перед настройками и питанием
                    let at = s.bar_right.iter().position(|x| x == "settings" || x == "power").unwrap_or(s.bar_right.len());
                    s.bar_right.insert(at, m.clone());
                }
                self.settings_changed();
            }
            Act::SWidgetSel(id) => {
                self.st.sel_widget = *id;
                self.st.widget_scripts = false;
                self.st.widget_layouts = false;
                self.st.widget_panels = false;
                self.st.widget_gallery = false;
            }
            Act::SWidgetAdd(k) => {
                let id = s.next_widget_id();
                let mut w = Widget::new(id, k);
                if *k == "net" {
                    w.style = "line".into();
                }
                if *k == "media" {
                    w.style = "bars".into();
                }
                // скрипт — первый из папки модулей
                if *k == "script" {
                    w.source = crate::plugins::scripts().into_iter().next().unwrap_or_default();
                }
                s.widgets.push(w);
                self.st.sel_widget = id;
                self.st.widget_gallery = false;
                self.settings_changed();
            }
            Act::SStartup(i) => {
                crate::startup::toggle(*i);
                self.dirty(&SKind::Modal);
            }
            Act::SPanelStyle(v) => {
                s.widget_panel = v.to_string();
                self.widgets_changed();
            }
            Act::SPanelLook(v) => {
                s.widget_look = v.to_string();
                self.widgets_changed();
            }
            Act::SLayout(i) => self.apply_layout(*i),
            Act::SLayoutSave => self.save_layout(),
            Act::SLayoutRestore => self.restore_layout(),
            Act::SWidgetAddScript(name) => {
                // готовый скрипт в папку (свой файл с этим именем остаётся как есть)
                let _ = crate::scriptlib::add(name);
                let id = s.next_widget_id();
                let mut w = Widget::new(id, "script");
                w.source = name.to_string();
                s.widgets.push(w);
                self.st.sel_widget = id;
                self.st.widget_scripts = false;
                self.settings_changed();
            }
            Act::SWidgetDel(id) => {
                s.widgets.retain(|w| w.id != *id);
                self.st.sel_widget = self.st.set.widgets.first().map(|w| w.id).unwrap_or(0);
                self.settings_changed();
            }
            Act::SWidgetOpt(id, key) => {
                if let Some(w) = s.widget_mut(*id) {
                    let v = crate::widgetkind::next_value(w, key);
                    widgets::set_opt(w, key, &v);
                }
                self.widgets_changed();
            }
            Act::SEdit => {
                self.close_all();
                self.set_editing(true);
            }
            Act::SResetPos => {
                let id = self.st.sel_widget;
                if let Some(w) = self.st.set.widget_mut(id) {
                    w.x = -1;
                    w.y = -1;
                    w.fx = -1.0;
                    w.fy = -1.0;
                }
                self.settings_changed();
            }
            Act::DockPin(id, _) => {
                let id = id.clone();
                self.st.set.dock_pins.retain(|p| *p != id);
                self.settings_changed();
            }
            Act::SField(k) => {
                self.st.field_focus = Some(k);
                if *k == "notif_muted" && self.st.muted_field.text.is_empty() {
                    let p = self.st.set.notif_muted.clone();
                    self.st.muted_field.set(&p);
                }
                if *k == "float_apps" && self.st.float_field.text.is_empty() {
                    let p = self.st.set.float_apps.clone();
                    self.st.float_field.set(&p);
                }
                if *k == "cast_hide" && self.st.cast_field.text.is_empty() {
                    let p = self.st.set.cast_hide.clone();
                    self.st.cast_field.set(&p);
                }
                if *k == "kb_layout" && self.st.keys.lay.text.is_empty() {
                    let p = self.st.set.kb_layout.clone();
                    self.st.keys.lay.set(&p);
                }
                if *k == "weather_city" && self.st.weather_field.text.is_empty() {
                    let p = self.st.set.weather_city.clone();
                    self.st.weather_field.set(&p);
                }
                if *k == "night_geo" && self.st.geo_field.text.is_empty() {
                    let p = self.st.set.night_geo.clone();
                    self.st.geo_field.set(&p);
                }
                if *k == "game_apps" && self.st.game_field.text.is_empty() {
                    let p = self.st.set.game_apps.clone();
                    self.st.game_field.set(&p);
                }
                if *k == "rec_dir" && self.st.rec_field.text.is_empty() {
                    let p = self.st.shot_cfg.record_dir.clone();
                    self.st.rec_field.set(&p);
                }
                if *k == "shot_dir" && self.st.shot_field.text.is_empty() {
                    let p = self.st.shot_cfg.dir.clone();
                    self.st.shot_field.set(&p);
                }
                if *k == "wall" && self.st.wall_field.text.is_empty() {
                    let p = self.st.set.wallpaper.clone();
                    self.st.wall_field.set(&p);
                }
            }
            _ => return self.wall_click(a) || self.keys_click(a) || self.update_click(a),
        }
        self.dirty(&SKind::Modal);
        true
    }

    pub(crate) fn apply_hex(&mut self) {
        let mut h = self.st.hex.text.trim().to_string();
        if !h.starts_with('#') {
            h.insert(0, '#');
        }
        if parse_hex(&h).is_some() {
            self.st.set.accent = h.to_lowercase();
            self.st.set.wall_accent = false;
            self.st.field_focus = None;
            self.settings_changed();
        }
    }
}
