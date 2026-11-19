// Что ставит `nemo-agents install`: файлы, вшитые в бинарь, и вопросы к человеку (nemo_px::install).

use nemo_px::install::{Asset, Choice, Spec};

pub const SPEC: Spec = Spec {
    name: "nemo-agents",
    version: env!("CARGO_PKG_VERSION"),
    about: ["окна паролей, показ экрана, автоподключение", "password windows, screen sharing, automount", "senhas, compartilhar a tela, montagem automática"], // i18n-ok
    deps: "polkit; optional: udisks2, gvfs, niri (screen sharing)",
    assets: &[],
    choices: &[Choice {
        ask: ["Зарегистрировать выбор экрана для показа (портал)?", "Register the screen-share picker (portal)?", "Registrar o seletor de compartilhamento de tela (portal)?"], // i18n-ok
        assets: &[
            Asset { path: "xdg-desktop-portal/portals/nemo-agents.portal", body: include_str!("../config/nemo-agents.portal") },
            Asset { path: "dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service", body: include_str!("../config/org.freedesktop.impl.portal.desktop.nemoagents.service") },
        ],
        default_app: false,
        note: None,
    }],
};
