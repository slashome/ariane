#!/usr/bin/make

.DEFAULT_GOAL := help
FILE_LIST = Makefile

# The toolchains Homebrew and rustup install outside the default PATH, when present
# (Node 26 is the version the CI and steiger need).
EXTRA_PATH = $(wildcard /opt/homebrew/opt/node@26/bin $(HOME)/.rustup/toolchains/stable-aarch64-apple-darwin/bin)
export PATH := $(subst $() ,:,$(strip $(EXTRA_PATH) $(PATH)))
# macOS' make 3.81 runs simple commands without a shell, ignoring the exported PATH.
SHELL := env PATH=$(PATH) /bin/bash

VERSION ?=

help: ## Show this help
	@for file in $(FILE_LIST) ; do \
		grep -E '(^[a-zA-Z0-9_-]+:.*?##.*$$)|(^##)' $$file | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[32m%-42s\033[0m %s\n", $$1, $$2}' | \
		sed -e 's/\[32m##/[33m/'; \
	done

##
## Development
## -----------
##

web-install: ## Install the front's dependencies
	pnpm -C web install --frozen-lockfile

web-build: web-install ## Build the front into web/dist, which the binary embeds
	pnpm -C web build

build: web-build ## Build the release binary into target/release/ariane
	cargo build --release --locked

install: web-build ## Build the front, then install the binary into ~/.cargo/bin
	cargo install --path . --locked

uninstall: ## Remove the binary installed by `make install`
	cargo uninstall ariane

##
## Checks
## ------
##

check: check-rust check-web ## Run every check the CI runs

check-rust: ## Format, clippy and tests on the Rust side
	cargo fmt --check
	RUSTFLAGS="-D warnings" cargo clippy --all-targets
	cargo test

check-web: web-install ## Typecheck, lint, Feature-Sliced Design and tests on the front
	pnpm -C web typecheck
	pnpm -C web lint
	pnpm -C web lint:fsd
	pnpm -C web test --run

##
## Release (Homebrew: brew install slashome/tap/ariane)
## -------
##

release-plan: ## Show what the next release would build and publish
	dist plan

release-generate: ## Regenerate .github/workflows/release.yml after a change to dist-workspace.toml
	dist generate

release: ## Bump to VERSION, commit, tag and push: the CI builds and publishes to the tap (make release VERSION=0.2.0)
	@test -n "$(VERSION)" || { echo "usage: make release VERSION=x.y.z"; exit 1; }
	@test -z "$$(git status --porcelain)" || { echo "the working tree is not clean"; exit 1; }
	@test "$$(git branch --show-current)" = develop || { echo "releases are cut from develop"; exit 1; }
	sed -i '' 's/^version = ".*"$$/version = "$(VERSION)"/' Cargo.toml
	cargo update --workspace --offline
	git commit -am "chore(release): v$(VERSION)"
	git tag "v$(VERSION)"
	git push origin develop "v$(VERSION)"

.PHONY: help web-install web-build build install uninstall check check-rust check-web release-plan release-generate release
