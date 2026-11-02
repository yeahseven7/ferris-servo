# ferris-servo

A **real-engine browser shell**: GTK4 chrome (tab strip + address bar + nav
buttons) wrapped around the [**Servo**](https://servo.org) web engine via the
[`servo-gtk`](https://github.com/nacho/servo-gtk) widget.

This is the sibling of [`ferris-browser`](https://github.com/yeahseven7/ferris-browser).
Where `ferris-browser` is a from-scratch *toy* engine (our own rasterizer, great
for learning, limited site compat), `ferris-servo` keeps a browser chrome but
delegates rendering + JavaScript to **Servo — a production-aimed engine that is
also written in Rust**. So this is the "all-Rust, real websites" lane: no
Chromium, no WebKit, no C web engine.

> **Status: prototype scaffold.** The Servo dependency tree is far too large to
> build in a small CI/sandbox box, so this crate is written against the proven
> `servo-gtk` example API but is **compile/run-verified on a real machine, not
> here**. Expect the first `cargo build` to take a long while (it builds Servo).

## What you get

- GTK4 window with a **tab strip** (reorderable tabs, per-tab close buttons)
- **address bar** (Enter to navigate; bare hosts get `https://`), synced to the
  active tab's real URI (redirects show up)
- back / forward / reload / new-tab buttons + a loading **spinner**
- page **title** reflected in each tab label
- `window.open` / `target=_blank` open as **new tabs**
- page zoom, script dialogs, file choosers, etc. come free from `servo-gtk`

## Build & run

Servo needs a real toolchain and a chunk of RAM; the first build is long.

**Arch / CachyOS:**

```bash
sudo pacman -S --needed git rust gtk4 libepoxy pkg-config python \
    fontconfig freetype2 libx11 libxkbcommon wayland libgl
git clone https://github.com/yeahseven7/ferris-servo
cd ferris-servo
cargo run --release
```

**Debian / Ubuntu:**

```bash
sudo apt install git build-essential pkg-config python3 \
    libgtk-4-dev libepoxy-dev libfontconfig-dev libfreetype-dev \
    libx11-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
cargo run --release
```

If Servo's own build complains about missing system libs, follow the
[Servo dev setup](https://book.servo.org/) for your distro — `servo-gtk` pulls
Servo in as a pinned git dependency, so whatever Servo needs, this needs.

## How it works

- `servo_gtk::run_as_runner_if_requested()` must be the first call in `main()`:
  `servo-gtk` runs the engine in a **subprocess** by re-executing our binary and
  hands off here when spawned as the runner.
- Each tab is a `servo_gtk::WebView` (a GTK4 widget). We observe `uri`, `title`,
  `is-loading` and `load-changed` to drive the chrome, and `create-web-view` to
  turn popups into tabs.
- The GL loader (`epoxy` + `libloading`) setup mirrors the upstream example.

## Roadmap / ideas

- reuse `ferris-browser`'s session ideas where Servo doesn't cover them
- optional YouTube → `mpv` hand-off (borrowed from `ferris-browser`) for
  DRM-protected streams Servo can't play in-page
- downloads, history, bookmarks
- swap the GTK4 chrome for our own winit/softbuffer chrome (the Verso pattern)
  if we want 100% Rust pixels end-to-end

## Credits

- [Servo](https://github.com/servo/servo) — the Rust web engine
- [servo-gtk](https://github.com/nacho/servo-gtk) — the GTK4 Servo widget this
  shell is built on
- Licensed MIT OR Apache-2.0 (Servo itself is MPL-2.0)
