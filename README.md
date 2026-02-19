# Ethereal Waves

A basic music player based on libcosmic. It's still very much a work in progress.

![Ethereal Waves - Dark Mode](https://github.com/LotusPetal392/ethereal-waves/blob/b970a4506b73b681b760d581c70f30d3a7eeed4b/screenshots/Ethereal%20Waves%20-%20Dark%20Mode.png?raw=true)
![Ethereal Waves - Light Mode](https://github.com/LotusPetal392/ethereal-waves/blob/b970a4506b73b681b760d581c70f30d3a7eeed4b/screenshots/Ethereal%20Waves%20-%20Light%20Mode.png?raw=true)

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
- [ ] Grid view
- [ ] More column options in list view
- [ ] Export playlist as .m3u
- [ ] Improved MPRIS support (much improved but not entirely complete)
- [ ] Sorting options
- [ ] Shuffle modes
- [ ] Condensed responsive layout
- [ ] More keyboard shortcuts
- [ ] Drag and drop support
- [ ] Playlist duplicate management
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
