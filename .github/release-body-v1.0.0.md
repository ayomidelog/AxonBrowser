## AxonBrowser v1.0.0

First public release of AxonBrowser.

### Highlights

- Browser automation for Chrome, Chromium, Edge, Firefox, and Camoufox on Linux
- Browser-native shell control through Chromium DevTools and Firefox WebDriver BiDi
- Real rendered-page interaction through AT-SPI and X11
- Single-command installer via `install.sh`
- Linux release bundle published on GitHub Releases

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/ayomidelog/AxonBrowser/main/install.sh | bash
```

### Release assets

- `install.sh`
- `axonbrowser-linux-x86_64.tar.gz`
- `axonbrowser-linux-x86_64.tar.gz.sha256`

### Notes

- This release targets Linux environments with X11 support.
- After installing the binary, run `axonbrowser install-deps` to install runtime dependencies.
