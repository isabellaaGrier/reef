Name:           reef
Version:        0.3.0
Release:        1%{?dist}
Summary:        Bash compatibility layer for fish shell
License:        MIT
URL:            https://github.com/ZStud/reef
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.85
BuildRequires:  cargo >= 1.85

Requires:       fish
Requires:       bash

%description
Reef makes bash syntax work seamlessly inside fish shell. No prefix commands,
no mode switching, no learning curve. You type bash — fish runs it. Includes
detection, AST translation, and bash passthrough with environment diffing.

%package tools
Summary:        Modern CLI tool wrappers for fish — grep→rg, find→fd, cat→bat, cd→zoxide
Requires:       fish
Recommends:     ripgrep fd-find sd dust procs eza bat zoxide

%description tools
Drop-in replacements that swap legacy coreutils for faster, modern alternatives.
Each wrapper fully mimics the original tool's flag interface, translating GNU
flags to their modern equivalents.

%prep
%autosetup -n %{name}-%{version}

%build
export RUSTUP_TOOLCHAIN=stable
cargo build --release --locked

%install
# Binary
install -Dm755 target/release/reef %{buildroot}%{_bindir}/reef

# Fish functions → vendor_functions.d
install -Dm644 fish/functions/export.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/export.fish
install -Dm644 fish/functions/unset.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/unset.fish
install -Dm644 fish/functions/declare.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/declare.fish
install -Dm644 fish/functions/local.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/local.fish
install -Dm644 fish/functions/readonly.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/readonly.fish
install -Dm644 fish/functions/shopt.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/shopt.fish
install -Dm644 fish/functions/fish_command_not_found.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/fish_command_not_found.fish

# conf.d
install -Dm644 fish/conf.d/reef.fish %{buildroot}%{_datadir}/fish/vendor_conf.d/reef.fish

# License
install -Dm644 LICENSE %{buildroot}%{_licensedir}/reef/LICENSE

# --- reef-tools ---
# Tool wrappers
install -Dm644 fish/functions/tools/grep.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/grep.fish
install -Dm644 fish/functions/tools/find.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/find.fish
install -Dm644 fish/functions/tools/sed.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/sed.fish
install -Dm644 fish/functions/tools/du.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/du.fish
install -Dm644 fish/functions/tools/ps.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/ps.fish
install -Dm644 fish/functions/tools/ls.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/ls.fish
install -Dm644 fish/functions/tools/cat.fish %{buildroot}%{_datadir}/fish/vendor_functions.d/cat.fish
install -Dm644 fish/conf.d/reef-tools.fish %{buildroot}%{_datadir}/fish/vendor_conf.d/reef-tools.fish

%check
cargo test --release --locked

%files
%license LICENSE
%{_bindir}/reef
%{_datadir}/fish/vendor_functions.d/export.fish
%{_datadir}/fish/vendor_functions.d/unset.fish
%{_datadir}/fish/vendor_functions.d/declare.fish
%{_datadir}/fish/vendor_functions.d/local.fish
%{_datadir}/fish/vendor_functions.d/readonly.fish
%{_datadir}/fish/vendor_functions.d/shopt.fish
%{_datadir}/fish/vendor_functions.d/fish_command_not_found.fish
%{_datadir}/fish/vendor_conf.d/reef.fish

%files tools
%license LICENSE
%{_datadir}/fish/vendor_functions.d/grep.fish
%{_datadir}/fish/vendor_functions.d/find.fish
%{_datadir}/fish/vendor_functions.d/sed.fish
%{_datadir}/fish/vendor_functions.d/du.fish
%{_datadir}/fish/vendor_functions.d/ps.fish
%{_datadir}/fish/vendor_functions.d/ls.fish
%{_datadir}/fish/vendor_functions.d/cat.fish
%{_datadir}/fish/vendor_conf.d/reef-tools.fish

%changelog
* Sun Feb 23 2026 Xavier B <zstud.dev@proton.me> - 0.3.0-1
- Persistence modes (state, full), confirm mode, library crate
- 498 unit tests + 11 doc tests, zero clippy warnings
