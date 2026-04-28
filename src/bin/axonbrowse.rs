use std::env;
use std::process::{exit, Command};

const HELP: &str = r#"axonbrowse — browser automation wrapper

Usage: axonbrowse <COMMAND>

Commands:
  launch       Launch a fresh Firefox/Camoufox window
  attach       Attach to the active Firefox/Camoufox window and print session state
  windows      List visible Firefox/Camoufox windows that can be attached to
  locate       Resolve a named browser-ui locator inside the first visible window
  screenshot   Capture a screenshot of the first visible Firefox/Camoufox window
  resize       Resize the active Firefox/Camoufox window
  focus        Focus a named browser-ui locator
  click        Click a named browser-ui locator
  type         Type text into a named browser-ui locator
  key          Press a key after focusing a named browser-ui locator
  press-enter   Press Enter after optionally focusing a named browser-ui locator
  hold         Hold a key down for a short duration in the Firefox/Camoufox window
  wait         Wait for a locator, page title change, or URL change
  goto         Navigate the current Firefox/Camoufox window by focusing the address bar, entering a URL, and pressing Enter
  back         Click the Firefox/Camoufox Back button
  forward      Click the Firefox/Camoufox Forward button
  reload       Click the Firefox/Camoufox Reload button
  new-tab      Click the Firefox/Camoufox New Tab button
  option       Click a button inside a Firefox/Camoufox-owned popup
  current      Print the current Firefox/Camoufox window, tab, and URL state
  tabs         List currently visible Firefox/Camoufox tabs
  page         Inspect and automate the rendered web page
  help         Print this message or the help of the given subcommand(s)
"#;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || matches!(args.first().map(String::as_str), Some("-h" | "--help" | "help")) {
        print!("{}", HELP);
        return;
    }

    let mut cmd = Command::new("axonbrowser");
    cmd.arg("camoufox");
    cmd.args(&args);

    let status = match cmd.status() {
        Ok(status) => status,
        Err(err) => {
            eprintln!("axonbrowse: failed to start axonbrowser command: {err}");
            exit(1);
        }
    };

    match status.code() {
        Some(code) => exit(code),
        None => exit(1),
    }
}
