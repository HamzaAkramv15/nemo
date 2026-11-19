# Сборка пакетов

[English](PACKAGING.md) · Русский

Заметки для пользователей Nix и сборщиков пакетов дистрибутивов: готовые бинарники и пакеты из них, flake, где Nemo ищет системные файлы и как отдать обновления пакетному менеджеру.

## Готовые бинарники

В каждом выпуске есть `nemo-X.Y.Z-x86_64-linux.tar.gz`: шелл и модули, собранные под старую glibc, поэтому они запускаются на любой системе x86_64 с glibc 2.34 или новее, какой бы дистрибутив ни стоял. Рядом лежат `.deb` на каждую программу и `SHA256SUMS`, подписанный ключом выпуска (`SHA256SUMS.sig`, пространство имён `nemo-release`, ключи в `keys/allowed_signers`). Установщик берёт их сам; делает их `tools/dist.sh` (без контейнера — под пакеты Debian 12, распакованные в `~/.cache/nemo/sysroot`; в контейнере Debian 12 — с `--no-sysroot`).

Внутри архива `stage.sh` раскладывает программы туда, где их ждёт пакет:

```sh
./stage.sh --prefix /usr --destdir "$pkgdir" --managed --only nemo-shell
```

`--only` берёт одну программу (`nemo-shell`, с ним его службы nemo-agents, `nemo-files` `nemo-view` `nemo-play` `nemo-shot` `nemo-greet`), без него — все; `--managed` оставляет метку `share/nemo/package-managed`, которая выключает обновления в этих же бинарниках. Службы `org.freedesktop.FileManager1` в пакете нет: в большинстве систем ею владеет Nautilus, а «показать в папке» для всех программ — личный выбор каждого (`nemo-files install` спросит и положит её в домашнюю папку).

| Система | Файл |
|---|---|
| Arch | `packaging/arch/PKGBUILD`, по пакету на программу |
| Debian, Ubuntu | файлы `.deb` из выпуска, `tools/dist.sh --deb` |
| Fedora, openSUSE | `packaging/fedora/nemo.spec` |
| Gentoo | `packaging/gentoo/nemo-bin-*.ebuild`, флаг USE на модуль |

В Ubuntu 26.04 нет пакета niri: `install.sh` соберёт его сам, а для `.deb` niri нужен откуда-то ещё. Пакет ставит ещё `/etc/xdg/autostart/nemo-shell.desktop`. Он запускает шелл (`nemo-shell autostart`) при первом входе, когда конфиг niri его ещё не запускает; первая настройка открывается сама, и её шаг niri подключает Nemo одной строкой include. После этого шелл запускает конфиг, а запись автозапуска ничего не делает. Без systemd niri записи автозапуска не выполняет: то же сделает `nemo niri connect`, один раз из терминала.

## Nix

Во flake есть пакет (шелл с его службами nemo-agents, nemo-files, nemo-view, nemo-play, nemo-shot и экран входа nemo-greet; модули — входы flake в версиях из `modules.lock`), модуль NixOS и модуль Home Manager. Модуль NixOS — системная половина `install.sh`: niri, службы, с которыми говорит шелл, порталы с диалогом файлов и выбором показа экрана от Nemo, шрифты, команды, которые запускает шелл, и экран входа. Модуль Home Manager — домашняя половина: конфиг niri, настройки шелла, kitty, fontconfig и программы по умолчанию.

```nix
{
  inputs.nemo.url = "github:HamzaAkramv15/nemo";
  inputs.nemo.inputs.nixpkgs.follows = "nixpkgs";

  # NixOS
  imports = [ nemo.nixosModules.default ];
  programs.nemo.enable = true;
  programs.nemo.loginScreen.user = "alice"; # чей шелл задаёт вид экрана входа

  # Home Manager
  imports = [ nemo.homeManagerModules.default ];
  programs.nemo.enable = true;
  programs.nemo.settings = { palette = "sakura"; lang = "en"; };
}
```

Параметры NixOS:

| Параметр | По умолчанию | Что |
|---|---|---|
| `programs.nemo.enable` | `false` | niri, NetworkManager, BlueZ, udisks, gvfs, polkit, профили питания, связка ключей, порталы, шрифты и команды шелла |
| `programs.nemo.loginScreen.enable` | `true` | greetd запускает nemo-greet внутри niri; выключенный оставляет другой менеджер входа |
| `programs.nemo.loginScreen.user` | `null` | пользователь, чей шелл пишет вид экрана входа (палитра, обои, язык, экраны, раскладки) |
| `programs.nemo.apps` | `true` | kitty, fastfetch, pavucontrol, btop, миниатюры видео, bsdtar |
| `programs.nemo.tools` | `true` | ночной свет, текст и QR-коды со скриншотов |
| `programs.nemo.appTheme` | `true` | qt6ct и adw-gtk3 для программ Qt и GTK в палитре |

Параметры Home Manager: `settings` (ключи `~/.config/nemo/shell.json`, сливаются при каждой активации: эти главнее, остальное из панели настроек остаётся), `niriConfig` (подключает nemo к `config.kdl` niri одной строкой include, `nemo niri connect`; конфиг-ссылку в Nix store не трогает, в него строку `include optional=true "~/.config/nemo/niri/nemo.kdl"` добавляешь сам), `kitty`, `defaultApplications`, `extraPackages`.

Шелл сам пишет свои файлы во время работы (настройки, клавиши, экраны, порталы), поэтому модули кладут их один раз и оставляют доступными для записи. nemo-niri, исправленный niri в своём репозитории, пакетом не собирается: пакетный niri — тот же стол без разрывов в играх, главного потока реального времени и правил слоёв для шторки трансляций. `nix flake check` собирает пакет и вычисляет модуль NixOS; CI запускает его при каждом пуше. Модули начинались с работы TheCodedKid в #1.

## Системные файлы

Шелл находит помощника polkit и бэкенд MTP из gvfs без жёстких путей FHS. На NixOS помощник polkit — setuid-обёртка `/run/wrappers/bin/polkit-agent-helper-1`, она идёт первой: копия помощника из `PATH` без бита setuid пароль не проверит. gvfs находится через `XDG_DATA_DIRS`. Старые пути в `/usr` остаются запасными.

Экран входа: greetd запускает `libexec/nemo/greet-session` от своего пользователя greeter (установщик кладёт его в `/usr/local/lib/nemo`). Он находит `bin/nemo-greet` от своей папки (`NEMO_GREET` задаёт другой), niri — в `PATH`, сеансы — в папках `wayland-sessions` из `XDG_DATA_DIRS`, а nemo-greet находит свои шрифты в `share/nemo-greet/fonts` рядом со своим бинарником (`NEMO_GREET_FONTS`). Нужны три папки: `/var/lib/nemo-greet/look` пользователя, чей шелл выкладывает вид, `/var/lib/nemo-greet/state` (0700) и `/var/lib/nemo-greet/log` пользователя greeter.

## Обновления через пакетный менеджер

Готовые бинарники: `--managed` у `stage.sh` (см. выше). Из исходников: соберите с признаком Cargo `package-managed` (flake так и делает):

```sh
cargo build --release --locked --workspace --features nemo-shell/package-managed --manifest-path shell/Cargo.toml
```

Такая сборка не проверяет обновления по расписанию, вкладка обновлений показывает заметку вместо кнопок, а `nemo update` (с любой подкомандой) пишет «Обновлениями управляет пакетный менеджер.» и выходит с 1. Обновление, переход на выпуск и откат отказываются раньше, чем тронут клон исходников или установленные бинари. Обновляйте и откатывайте пакетным менеджером. Сборка без признака сохраняет обновления.

Страница модулей в первой настройке тогда только показывает, что стоит. Не запускайте `install.sh` поверх установки, которой владеет пакетный менеджер.
