# axonbrowser

Browser automation via the accessibility tree — Chrome, Edge, Firefox, and Camoufox.

Axonbrowser drives real browser windows through the OS accessibility bus (AT‑SPI).
It operates at two levels: the **browser shell** (address bar, tabs, resize,
navigation) and the **rendered page** (find, click, type, wait, form helpers,
iframes, screenshots).

---

## Runtime Requirements

- Linux desktop session (X11)
- Chrome/Chromium, Microsoft Edge, or Firefox
- AT‑SPI accessibility bus enabled
- `xdotool`, `wmctrl` (fallback input / window control)
- `xwd` (X11 window dump, used by screenshot commands)
- ImageMagick `convert` (XWD→PNG conversion and screenshot cropping)

On headless machines, `axonbrowser` now auto-bootstraps its own X11 session with
`Xvfb`, `openbox`, and `x11vnc` when no working `DISPLAY` is available. The
session is isolated under `~/.cache/axonbrowser/headless` and the VNC server is bound
to `localhost` on an axonbrowser-owned port.

---

## Build

```bash
cargo build --release
# binary lands at target/release/axonbrowser
```

Local source builds require a Rust toolchain.

## Install

```bash
# Install from a release bundle or from a local checkout
scripts/install-axonbrowser.sh

# Optional local browser installers
scripts/install-chrome-local.sh
scripts/install-firefox-local.sh
scripts/install-edge-local.sh
scripts/install-camoufox.sh
```

`scripts/install-runtime-deps.sh` installs the X11/accessibility/runtime
packages axonbrowser needs. On headless hosts it also installs the VNC/Xvfb pieces.

If you install from the release tarball, Rust is not required. Extract the bundle,
run `scripts/install-axonbrowser.sh`, and then use `axonbrowser --help`.

---

## Quick start

```bash
# Launch a browser and navigate
axonbrowser chrome launch
axonbrowser chrome goto example.com

# Page interactions
axonbrowser chrome page find "Heading:Example Domain"
axonbrowser chrome page click "Button:Sign in"
axonbrowser chrome page type "Text Box:Email" "user@example.com"
axonbrowser chrome page wait --text "Welcome"
axonbrowser chrome page screenshot artifacts/shot.png

# Edge, Firefox, and Camoufox use the same command surface
axonbrowser edge launch
axonbrowser firefox launch
axonbrowser camoufox launch
axonbrowser firefox goto example.com
```

Run `axonbrowser --help` for the full command tree.

---

## Browser commands

The core browser commands are shared by all four browser entry points (`chrome`,
`edge`, `firefox`, `camoufox`).

### Launch / attach

```bash
axonbrowser chrome launch
axonbrowser chrome launch https://example.com
axonbrowser chrome launch --profile /tmp/my-profile
axonbrowser chrome attach
axonbrowser chrome attach --json
```

### Navigate

```bash
axonbrowser chrome goto example.com
axonbrowser chrome goto https://example.com --new-tab
axonbrowser chrome back
axonbrowser chrome forward
axonbrowser chrome reload
axonbrowser chrome new-tab
```

### Window state

```bash
axonbrowser chrome current
axonbrowser chrome current --json
axonbrowser chrome resize --preset desktop    # 1440×900
axonbrowser chrome resize --preset tablet     # 1024×768
axonbrowser chrome resize --preset mobile     # 430×932
axonbrowser chrome resize --width 1280 --height 900
```

### Tabs

```bash
axonbrowser chrome tabs list
axonbrowser chrome tabs switch 1
axonbrowser chrome tabs switch --title example
axonbrowser chrome tabs close
axonbrowser chrome tabs close --index 0
```

### Browser shell interactions

```bash
axonbrowser chrome locate address-bar
axonbrowser chrome focus address-bar
axonbrowser chrome click reload
axonbrowser chrome type address-bar https://example.com
axonbrowser chrome key address-bar ctrl+l
axonbrowser chrome press-enter
axonbrowser chrome hold ctrl --duration-ms 500
axonbrowser chrome screenshot artifacts/chrome.png
axonbrowser chrome screenshot --active artifacts/active.png
```

### Browser popup options

Chrome and Edge expose the same prompt names directly. Firefox uses the same
command names, but maps them onto Firefox-native labels where those prompts
differ.

```bash
axonbrowser chrome option leave-site cancel
axonbrowser chrome option leave-site leave
axonbrowser chrome option save-password never
axonbrowser chrome option save-password save

axonbrowser edge option leave-site cancel
axonbrowser edge option save-password never

axonbrowser firefox option leave-site cancel
axonbrowser firefox option save-password never

axonbrowser camoufox option leave-site cancel
axonbrowser camoufox option save-password never
```

---

## Page commands

Page commands target the rendered web content inside the browser.

### Selectors

```
Role:Name          exact role + name    "Heading:Example Domain"
Role               role only            "Push Button"
~name              name contains        "~continue"
Role~name          role + contains      "Button~submit"
```

Common role aliases: `Button` → Push Button, `Text Box` / `Input` → Entry,
`Frame` / `Iframe` → frame scope, `Check Box`, `Radio Button`, `Combo Box`.

Use `--nth <n>` to pick the nth match (0-based, default 0). Chain selectors to scope.

### Inspect

```bash
axonbrowser chrome page inspect
axonbrowser chrome page frames
axonbrowser chrome page inspect --frame "Frame:Checkout"
```

### Find, count, read

```bash
axonbrowser chrome page find "Heading:Example Domain"
axonbrowser chrome page count "Push Button"
axonbrowser chrome page read "Heading:Example Domain"
axonbrowser chrome page read --value "Text Box:Email"
```

### Click, focus, hover

```bash
axonbrowser chrome page focus "Text Box:Email"
axonbrowser chrome page click "Button:Submit"
axonbrowser chrome page hover "Button:Submit"
axonbrowser chrome page double-click "Button:Submit"
axonbrowser chrome page right-click "Button:Submit"
axonbrowser chrome page click --nth 2 "Button:Edit"
axonbrowser chrome page click --frame "Frame:Checkout" "Button:Pay"
```

### Type, keys

```bash
axonbrowser chrome page type "Text Box:Email" "user@example.com"
axonbrowser chrome page key "Text Box:Email" Tab
axonbrowser chrome page press-enter "Button:Submit"
```

### Form helpers

```bash
axonbrowser chrome page check "Check Box:Accept Terms"
axonbrowser chrome page uncheck "Check Box:Accept Terms"
axonbrowser chrome page check "Radio Button:Pro Plan"
axonbrowser chrome page select-option "Combo Box:Country" "Canada"
axonbrowser chrome page upload "Button~Upload File" ./resume.pdf
```

### Scroll

```bash
axonbrowser chrome page scroll --direction down --amount 3
axonbrowser chrome page scroll --direction up --amount 1
axonbrowser chrome page scroll "Button:Dismiss Notice" --into-view
```

### Wait

```bash
axonbrowser chrome page wait --text "Submitted"
axonbrowser chrome page wait --title-contains "Dashboard"
axonbrowser chrome page wait --url-contains "dashboard.html"
axonbrowser chrome page wait "Button:Submit"
axonbrowser chrome page wait --state focused "Text Box:Email"
axonbrowser chrome page wait --text "Loading" --disappear
```

### Action + wait flows

```bash
axonbrowser chrome page click-and-wait "Link:Continue" --title-contains "Step 2"
axonbrowser chrome page submit-and-wait "Button:Submit" --text "Submitted"
axonbrowser chrome page submit-and-wait "Button:Submit" --url-contains "done.html"
```

### Screenshots

```bash
axonbrowser chrome page screenshot artifacts/page.png
axonbrowser chrome page screenshot artifacts/button.png "Button:Submit"
axonbrowser chrome page screenshot artifacts/frame.png --frame "Frame:Checkout" "Button:Pay"
```

---

## Development

```bash
# Run unit tests
cargo test

# Smoke tests against a live VNC desktop
scripts/vnc-chrome-smoke.sh
scripts/vnc-page-smoke.sh

# Headless bootstrap happens automatically inside the CLI now
cargo run -- chrome launch
```

The demo site used for page smoke tests lives under `artifacts/page-test-site/`.

---

## Repo layout

```
src/
  main.rs                        entry point and command dispatch
  cli.rs                         clap CLI definitions
  selector.rs                    selector parsing and matching
  chrome/                        Chrome/Chromium automation
    launch.rs, attach.rs
    goto.rs, current.rs
    locators.rs, resize.rs
    screenshot.rs
    tabs/                        tab listing, switching, closing
    actions/                     browser shell focus/click/type/key/hold
    page/
      find.rs, wait.rs
      screenshot.rs
      actions/                   click, type, form, scroll, upload, pointer
  edge/                          Edge automation (mirrors chrome/)
  firefox/                       Firefox automation (mirrors chrome/)
  inspect.rs                     AT-SPI tree inspection
  window.rs                      X11 window helpers
scripts/
  use-vnc-session.sh
  vnc-chrome-smoke.sh
  vnc-page-smoke.sh
  vnc-chrome-flow.sh
```
