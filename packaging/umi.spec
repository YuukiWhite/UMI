%global debug_package %{nil}

Name:           umi
Version:        1.3.1
Release:        1%{?dist}
Summary:        UMI — Ambient Dynamic Island Companion for KDE Plasma Wayland
License:        MIT
URL:            https://github.com/YuukiWhite/UMI
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.85
BuildRequires:  gtk4-devel
BuildRequires:  gtk4-layer-shell-devel
BuildRequires:  sqlite-devel
BuildRequires:  dbus-devel

Requires:       gtk4
Requires:       gtk4-layer-shell
Requires:       sqlite-libs
Requires:       xdg-desktop-portal-kde
Requires:       kf6-kwallet

%description
Pure Rust Dynamic Island desktop overlay for Fedora KDE Plasma (Wayland / KWin)
featuring 18 interactive notch states, MPRIS2 + LRCLIB synced lyrics, offline
Kemenag RI prayer times with Sun Arc, Focus Pomodoro, Hydration tracker,
Wayland drag-and-drop File Shelf, Klipper clipboard history, Ask UMI AI +
Gemini Vision Smart OCR, AI subscription quota tracker, and 7-screen Settings.

%prep
%autosetup

%build
cargo build --release --locked

%install
install -Dm0755 target/release/umi %{buildroot}%{_bindir}/umi
install -Dm0644 assets/icons/umi-logo.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/org.umi.Notch.svg
mkdir -p %{buildroot}%{_datadir}/applications
cat > %{buildroot}%{_datadir}/applications/org.umi.Notch.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=UMI
GenericName=Ambient Notch Companion
Comment=Dynamic Island desktop overlay for KDE Plasma Wayland
Exec=/usr/bin/umi
Icon=org.umi.Notch
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=org.umi.Notch
EOF

%files
%{_bindir}/umi
%{_datadir}/icons/hicolor/scalable/apps/org.umi.Notch.svg
%{_datadir}/applications/org.umi.Notch.desktop

%changelog
* Thu Nov 26 2026 Farid <yuukiwhite@fedora> - 1.3.1-1
- Complete Phase 1-5 Pure Rust release for Fedora 44 KDE Plasma Wayland
