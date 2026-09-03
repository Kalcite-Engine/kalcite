PREFIX ?= /usr/local
DESTDIR ?=
BINDIR := $(DESTDIR)$(PREFIX)/bin

.PHONY: build release test install uninstall deb rpm

build:
	cargo build -p kalcite-cli

release:
	cargo build --release -p kalcite-cli

test:
	cargo fmt --all -- --check
	cargo test --workspace

install: release
	install -Dm755 target/release/kalcite $(BINDIR)/kalcite

uninstall:
	rm -f $(BINDIR)/kalcite

deb: release
	@test -n "$(VERSION)" || (echo "use: make deb VERSION=0.14.0"; exit 2)
	rm -rf dist/deb
	mkdir -p dist/deb/DEBIAN dist/deb$(PREFIX)/bin
	install -Dm755 target/release/kalcite dist/deb$(PREFIX)/bin/kalcite
	printf 'Package: kalcite\nVersion: $(VERSION)\nArchitecture: amd64\nMaintainer: Kalcite Engine\nDescription: Kalcite compiler and project CLI\n' > dist/deb/DEBIAN/control
	dpkg-deb --build dist/deb dist/kalcite_$(VERSION)_amd64.deb

rpm: release
	@echo "Use packaging/rpm/kalcite.spec with rpmbuild -bb; generated RPMs belong in dist/."
