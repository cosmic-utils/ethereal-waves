# Ethereal Waves

A basic music player based on libcosmic. It's still very much a work in progress.

![Ethereal Waves - Dark Mode](https://github.com/cosmic-utils/ethereal-waves/blob/e376ce81ddfe7b3fb4357b9e882d33586102b13b/screenshots/Ethereal%20Waves%20-%20Dark%20Mode.png?raw=true)
![Ethereal Waves - Light Mode](https://github.com/cosmic-utils/ethereal-waves/blob/e376ce81ddfe7b3fb4357b9e882d33586102b13b/screenshots/Ethereal%20Waves%20-%20Light%20Mode.png?raw=true)

## Supported Formats

- MP3
- M4A
- Ogg
- Opus
- Flac
- Wav

## Planned Features

Non-exhaustive list of planned features in no particular order:

- [x] Gapless playback
- [x] Crossfading between tracks
- [x] Grid view
- [x] More column options in list view (calling this complete for the time being)
- [x] Import / Export .m3u playlists
- [ ] MPRIS support (much improved but not entirely complete)
- [ ] Sorting options
- [ ] Shuffle modes
- [x] Condensed responsive layout (possibly may build on this later)
- [x] Drag and drop support (your milage may vary outside of cosmic-comp)
- [x] Playlist duplicate management
- [ ] Partial update (Only add new tracks)

## Keybindings

- `Ctrl + U`: Update Library
- `Ctrl + Q`: Quit
- `Ctrl + N`: New Playlist
- `F2`: Rename Playlist
- `Ctrl + Up`: Move Playlist Up
- `Ctrl + Down`: Move Playlist Down
- `Ctrl + =`: Zoom In
- `Ctrl + -`: Zoom Out
- `PageUp`: Scroll Up
- `PageDown`: Scroll Down
- `Ctrl + ,`: Settings
- `Ctrl + A`: Select All
- `Ctrl + click`: Select
- `Shift + click`: Select Range
- `F1`: Track Info
- `m`: Toggle Mute
- `-`: Volume Down
- `=`: Volume Up

## Installation

This project uses `just` for building. To run development mode:

```
just run-dev
```

To install:

```
sudo apt install just
```

```
just build-release
sudo just install
```
