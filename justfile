# Name of the application's binary.
name := 'ethereal-waves'
# The unique ID of the application.
appid := 'com.galacticpirateradio.ethereal-waves'

# Path to root file system, which defaults to `/`.
rootdir := ''
# The prefix for the `/usr` directory.
prefix := '/usr'
# The location of the cargo target directory.
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

# Application's appstream metadata
appdata := appid + '.metainfo.xml'
# Application's desktop entry
desktop := appid + '.desktop'
# Application's icon.
icon-svg := appid + '.svg'

# Install destinations
base-dir := absolute_path(clean(rootdir / prefix))
appdata-dst := base-dir / 'share' / 'appdata' / appdata
bin-dst := base-dir / 'bin' / name
desktop-dst := base-dir / 'share' / 'applications' / desktop
icons-dst := base-dir / 'share' / 'icons' / 'hicolor'
icon-svg-dst := icons-dst / 'scalable' / 'apps'

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# Removes vendored dependencies
clean-vendor:
    rm -rf .cargo vendor vendor.tar

# `cargo clean` and removes vendored dependencies
clean-dist: clean clean-vendor

# Compiles with debug profile
build-debug *args:
    cargo build {{ args }}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Runs a clippy check
check *args:
    cargo clippy --all-features {{ args }} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Run the application for testing purposes
run *args:
    env RUST_BACKTRACE=full cargo run --release {{ args }}

# Run with dev profile
run-dev:
    cargo run

# Installs files
install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{ bin-dst }}
    install -Dm0644 {{ 'resources' / desktop }} {{ desktop-dst }}
    install -Dm0644 {{ 'resources' / appdata }} {{ appdata-dst }}
    install -Dm0644 {{ 'resources' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon-svg }} {{ icon-svg-dst }}

# Uninstalls installed files
uninstall:
    rm {{ bin-dst }} {{ desktop-dst }} {{ icon-svg-dst / icon-svg }}

# Vendor dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Regenerate flatpak cargo sources
flatpak-sources:
    mkdir -p .flatpak-cargo-cache .flatpak-cargo-home
    XDG_CACHE_HOME={{invocation_directory() / '.flatpak-cargo-cache'}} HOME={{invocation_directory() / '.flatpak-cargo-home'}} flatpak-cargo-generator Cargo.lock -o cargo-sources.json

# Build the flatpak using local manifest
flatpak-build:
    flatpak-builder --user --install-deps-from=flathub --force-clean build-dir com.galacticpirateradio.ethereal-waves.yml

# Run the app from local flatpak build dir
flatpak-run:
    if [ -z "${WAYLAND_DISPLAY:-}" ] && [ -z "${DISPLAY:-}" ]; then echo "No graphical display detected. Run this from a desktop session terminal."; exit 1; fi
    flatpak-builder --run --socket=wayland --socket=fallback-x11 --socket=pulseaudio --share=ipc --device=dri --filesystem=xdg-data/fonts:ro --filesystem=~/.fonts:ro --filesystem=xdg-config/fontconfig:ro --env=WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-} --env=WAYLAND_SOCKET=${WAYLAND_SOCKET:-} --env=DISPLAY=${DISPLAY:-} build-dir com.galacticpirateradio.ethereal-waves.yml ethereal-waves

# Bump cargo version, create git commit, and create tag
tag version:
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{ version }}"/' '{}' \; -exec git add '{}' \;
    cargo check
    cargo clean
    git add Cargo.lock
    git commit -m 'release: {{ version }}'
    git commit --amend
    git tag -a {{ version }} -m ''
