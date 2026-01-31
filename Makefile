# Version
VERSION = $(shell cat VERSION 2>/dev/null || echo "1.0.0")

# Installation directories
PREFIX ?= /usr/local
DESTDIR ?=

.PHONY: all api clean install install-all install-api install-pw-param install-link-nodes install-api-man install-config deb deb-clean

all: api

api:
	@echo "Building API server and tools..."
	cargo build --release --bin pipewire-api --bin pw-param --bin link-nodes

clean:
	@echo "Cleaning Rust build artifacts..."
	cargo clean
	@echo "Cleaning Debian build artifacts..."
	rm -rf debian/.debhelper debian/pipewire-api debian/tmp debian/files debian/*.substvars debian/*.debhelper.log debian/debhelper-build-stamp
	rm -f debian/*.log debian/*.debhelper

install: install-pw-param install-link-nodes install-api-man install-config install-api

install-all: install-pw-param install-link-nodes install-api-man install-config install-api

install-pw-param:
	@echo "Installing pw-param tool..."
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp target/release/pw-param $(DESTDIR)$(PREFIX)/bin/
	@echo "Installed pw-param to $(DESTDIR)$(PREFIX)/bin/pw-param"

install-link-nodes:
	@echo "Installing link-nodes tool..."
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp target/release/link-nodes $(DESTDIR)$(PREFIX)/bin/
	@echo "Installed link-nodes to $(DESTDIR)$(PREFIX)/bin/link-nodes"

install-config:
	@echo "Installing default config files..."
	mkdir -p $(DESTDIR)/etc/pipewire-api
	cp link-rules.conf $(DESTDIR)/etc/pipewire-api/
	cp device-volumes.conf $(DESTDIR)/etc/pipewire-api/
	@echo "Installed default config to $(DESTDIR)/etc/pipewire-api/link-rules.conf"
	@echo "Installed default config to $(DESTDIR)/etc/pipewire-api/device-volumes.conf"

install-api-man: pipewire-api.1
	@echo "Installing man pages..."
	mkdir -p $(DESTDIR)$(PREFIX)/share/man/man1
	mkdir -p $(DESTDIR)$(PREFIX)/share/man/man5
	cp pipewire-api.1 $(DESTDIR)$(PREFIX)/share/man/man1/
	cp device-volumes.conf.5 $(DESTDIR)$(PREFIX)/share/man/man5/
	@if command -v gzip >/dev/null 2>&1; then \
		gzip -f $(DESTDIR)$(PREFIX)/share/man/man1/pipewire-api.1; \
		gzip -f $(DESTDIR)$(PREFIX)/share/man/man5/device-volumes.conf.5; \
		echo "Installed man pages to $(DESTDIR)$(PREFIX)/share/man/"; \
	else \
		echo "Installed man pages to $(DESTDIR)$(PREFIX)/share/man/"; \
	fi

install-api: target/release/pipewire-api
	@echo "Installing API server..."
	@echo "Note: Stop services first with: systemctl --user stop pipewire-api pipewire wireplumber"
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	cp target/release/pipewire-api $(DESTDIR)$(PREFIX)/bin/
	@echo "Installing systemd user unit..."
	@if [ -n "$$SUDO_USER" ]; then \
		USER_HOME=$$(getent passwd $$SUDO_USER | cut -d: -f6); \
		mkdir -p $$USER_HOME/.config/systemd/user; \
		chown $$SUDO_USER:$$(id -gn $$SUDO_USER) $$USER_HOME/.config/systemd/user; \
		echo "[Unit]" > $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "Description=PipeWire REST API Server" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "After=pipewire.service" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "Requires=pipewire.service" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "[Service]" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "Type=simple" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "ExecStartPre=/bin/sleep 2" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "ExecStart=$(PREFIX)/bin/pipewire-api" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "Restart=on-failure" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "RestartSec=5" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "[Install]" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		echo "WantedBy=default.target" >> $$USER_HOME/.config/systemd/user/pipewire-api.service; \
		chown -R $$SUDO_USER:$$(id -gn $$SUDO_USER) $$USER_HOME/.config/systemd/user; \
		echo "Installed systemd unit to $$USER_HOME/.config/systemd/user/pipewire-api.service"; \
	else \
		mkdir -p $$HOME/.config/systemd/user; \
		echo "[Unit]" > $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "Description=PipeWire REST API Server" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "After=pipewire.service" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "Requires=pipewire.service" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "[Service]" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "Type=simple" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "ExecStartPre=/bin/sleep 2" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "ExecStart=$(PREFIX)/bin/pipewire-api" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "Restart=on-failure" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "RestartSec=5" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "[Install]" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "WantedBy=default.target" >> $$HOME/.config/systemd/user/pipewire-api.service; \
		echo "Installed systemd unit to $$HOME/.config/systemd/user/pipewire-api.service"; \
	fi
	@echo "Installed API server to $(DESTDIR)$(PREFIX)/bin/pipewire-api"
	@echo "Reload systemd and enable with: systemctl --user daemon-reload && systemctl --user enable --now pipewire-api"

# Debian packaging
deb:
	@echo "Building Debian package version $(VERSION)..."
	dpkg-buildpackage -us -uc -b

deb-clean:
	@echo "Cleaning Debian build artifacts..."
	rm -rf debian/.debhelper debian/pipewire-api debian/files debian/*.substvars debian/*.debhelper.log
	rm -f ../pipewire-api_*.deb ../pipewire-api_*.buildinfo ../pipewire-api_*.changes
