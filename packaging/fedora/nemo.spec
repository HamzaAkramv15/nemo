# nemo from the release's ready binaries (tools/dist.sh), one subpackage per program.
# rpmbuild -bb packaging/fedora/nemo.spec, the archive in ~/rpmbuild/SOURCES
%global debug_package %{nil}
%global __brp_strip %{nil}

Name:           nemo
Version:        3.0.0
Release:        1%{?dist}
Summary:        Lightweight pixel-art Wayland desktop for the niri compositor
License:        MIT
URL:            https://github.com/HamzaAkramv15/nemo
Source0:        https://github.com/HamzaAkramv15/nemo/releases/download/v%{version}/nemo-%{version}-x86_64-linux.tar.gz
ExclusiveArch:  x86_64

%description
The nemo shell for niri and its modules, each its own package.

%package shell
Summary:        Bar, launcher, dock, lock screen, notifications and widgets for niri
Requires:       niri pam polkit libxkbcommon
Recommends:     nemo-files nemo-view nemo-play nemo-shot udisks2 gvfs grim wl-clipboard cliphist playerctl brightnessctl
%description shell
The nemo shell.

%package files
Summary:        File manager and the file dialog of every app
Requires:       libxkbcommon
Recommends:     ffmpegthumbnailer bsdtar udisks2 gvfs
%description files
nemo-files.

%package view
Summary:        Image viewer with quick edits
Requires:       libxkbcommon
%description view
nemo-view.

%package play
Summary:        Video and music player
Requires:       mpv-libs libxkbcommon
%description play
nemo-play.

%package shot
Summary:        Screenshots, marks, text and QR
Requires:       grim libxkbcommon
Recommends:     tesseract zbar
%description shot
nemo-shot.

%package greet
Summary:        Login screen for greetd
Requires:       greetd niri libxkbcommon
%description greet
nemo-greet.

%prep
%setup -q -n nemo-%{version}-x86_64-linux

%install
./stage.sh --prefix %{_prefix} --destdir %{buildroot} --managed

%files shell
%{_bindir}/nemo
%{_bindir}/nemo-shell
%{_bindir}/nemo-agents
%{_datadir}/xdg-desktop-portal/portals/nemo-agents.portal
%{_datadir}/dbus-1/services/org.freedesktop.impl.portal.desktop.nemoagents.service
%{_sysconfdir}/xdg/autostart/nemo-shell.desktop
%{_datadir}/fonts/nemo
%{_datadir}/nemo
%{_datadir}/licenses/nemo-shell

%files files
%{_bindir}/nemo-files
%{_datadir}/applications/nemo-files.desktop
%{_datadir}/xdg-desktop-portal/portals/nemo.portal
%{_datadir}/dbus-1/services/org.freedesktop.impl.portal.desktop.nemo.service
%{_datadir}/licenses/nemo-files

%files view
%{_bindir}/nemo-view
%{_datadir}/applications/nemo-view.desktop
%{_datadir}/licenses/nemo-view

%files play
%{_bindir}/nemo-play
%{_datadir}/applications/nemo-play.desktop
%{_datadir}/licenses/nemo-play

%files shot
%{_bindir}/nemo-shot
%{_datadir}/licenses/nemo-shot

%files greet
%{_bindir}/nemo-greet
%{_prefix}/libexec/nemo/greet-session
%{_datadir}/nemo-greet
%{_datadir}/licenses/nemo-greet
