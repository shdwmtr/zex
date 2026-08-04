PREFIX ?= /usr/local
DESTDIR ?=

BIN_NAME := zex
BIN_ALIAS := zexplorer
CARGO_PROFILE := optimized-release
TARGET_BIN := target/$(CARGO_PROFILE)/$(BIN_NAME)

bindir := $(DESTDIR)$(PREFIX)/bin
desktopdir := $(DESTDIR)$(PREFIX)/share/applications
pixmapdir := $(DESTDIR)$(PREFIX)/share/pixmaps

APP_DIR := target/macos/Zex.app

.PHONY: build install uninstall app app-install app-uninstall

build:
	cargo build --profile $(CARGO_PROFILE)

# macOS: assemble target/optimized-release/zex into a proper Zex.app bundle.
app: build
	./packaging/macos/build-app.sh

app-install: app
	rm -rf "/Applications/Zex.app"
	cp -R "$(APP_DIR)" "/Applications/Zex.app"

app-uninstall:
	rm -rf "/Applications/Zex.app"

install:
	@test -f $(TARGET_BIN) || { echo "error: $(TARGET_BIN) not found; run 'make build' first (without sudo)"; exit 1; }
	install -Dm755 $(TARGET_BIN) $(bindir)/$(BIN_NAME)
	ln -sf $(BIN_NAME) $(bindir)/$(BIN_ALIAS)
	install -Dm644 packaging/zex.desktop $(desktopdir)/zex.desktop
	install -Dm644 assets/icon.png $(pixmapdir)/zex.png
ifeq ($(DESTDIR),)
	-update-desktop-database $(PREFIX)/share/applications 2>/dev/null
endif

uninstall:
	rm -f $(bindir)/$(BIN_NAME)
	rm -f $(bindir)/$(BIN_ALIAS)
	rm -f $(desktopdir)/zex.desktop
	rm -f $(pixmapdir)/zex.png
ifeq ($(DESTDIR),)
	-update-desktop-database $(PREFIX)/share/applications 2>/dev/null
endif
