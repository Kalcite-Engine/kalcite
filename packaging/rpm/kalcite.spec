Name:           kalcite
Version:        0.14.0
Release:        1%{?dist}
Summary:        Kalcite compiler and project CLI
License:        MIT
BuildRequires:  cargo, rust

%description
Kalcite ahead-of-time compiler and project command-line interface.

%prep
%autosetup -n kalcite-%{version}

%build
cargo build --release -p kalcite-cli

%install
install -Dm755 target/release/kalcite %{buildroot}%{_bindir}/kalcite

%files
%license LICENSE
%{_bindir}/kalcite
