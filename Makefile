PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
FISHDIR ?= $(PREFIX)/share/fish/vendor_functions.d
FISH_CONFDIR ?= $(PREFIX)/share/fish/vendor_conf.d
FISH_TOOLSDIR ?= $(FISHDIR)

.PHONY: all build install install-tools uninstall clean

all: build

build:
	cargo build --release

install: build
	install -Dm755 target/release/reef $(DESTDIR)$(BINDIR)/reef
	# Core fish functions
	install -Dm644 fish/functions/export.fish $(DESTDIR)$(FISHDIR)/export.fish
	install -Dm644 fish/functions/unset.fish $(DESTDIR)$(FISHDIR)/unset.fish
	install -Dm644 fish/functions/declare.fish $(DESTDIR)$(FISHDIR)/declare.fish
	install -Dm644 fish/functions/local.fish $(DESTDIR)$(FISHDIR)/local.fish
	install -Dm644 fish/functions/readonly.fish $(DESTDIR)$(FISHDIR)/readonly.fish
	install -Dm644 fish/functions/shopt.fish $(DESTDIR)$(FISHDIR)/shopt.fish
	install -Dm644 fish/functions/source.fish $(DESTDIR)$(FISHDIR)/source.fish
	install -Dm644 fish/functions/fish_command_not_found.fish $(DESTDIR)$(FISHDIR)/fish_command_not_found.fish
	# conf.d
	install -Dm644 fish/conf.d/reef.fish $(DESTDIR)$(FISH_CONFDIR)/reef.fish

install-tools: install
	install -Dm644 fish/functions/tools/grep.fish $(DESTDIR)$(FISH_TOOLSDIR)/grep.fish
	install -Dm644 fish/functions/tools/find.fish $(DESTDIR)$(FISH_TOOLSDIR)/find.fish
	install -Dm644 fish/functions/tools/sed.fish $(DESTDIR)$(FISH_TOOLSDIR)/sed.fish
	install -Dm644 fish/functions/tools/du.fish $(DESTDIR)$(FISH_TOOLSDIR)/du.fish
	install -Dm644 fish/functions/tools/ps.fish $(DESTDIR)$(FISH_TOOLSDIR)/ps.fish

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/reef
	rm -f $(DESTDIR)$(FISHDIR)/export.fish
	rm -f $(DESTDIR)$(FISHDIR)/unset.fish
	rm -f $(DESTDIR)$(FISHDIR)/declare.fish
	rm -f $(DESTDIR)$(FISHDIR)/local.fish
	rm -f $(DESTDIR)$(FISHDIR)/readonly.fish
	rm -f $(DESTDIR)$(FISHDIR)/shopt.fish
	rm -f $(DESTDIR)$(FISHDIR)/source.fish
	rm -f $(DESTDIR)$(FISHDIR)/fish_command_not_found.fish
	rm -f $(DESTDIR)$(FISH_CONFDIR)/reef.fish
	# Tool wrappers
	rm -f $(DESTDIR)$(FISH_TOOLSDIR)/grep.fish
	rm -f $(DESTDIR)$(FISH_TOOLSDIR)/find.fish
	rm -f $(DESTDIR)$(FISH_TOOLSDIR)/sed.fish
	rm -f $(DESTDIR)$(FISH_TOOLSDIR)/du.fish
	rm -f $(DESTDIR)$(FISH_TOOLSDIR)/ps.fish

clean:
	cargo clean
