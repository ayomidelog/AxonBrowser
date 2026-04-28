use anyhow::Result;

use crate::browser_options::{
    self, BrowserChoiceSpec, BrowserOptionChoice, BrowserOptionProfile, BrowserOptionPrompt,
    BrowserPromptSpec,
};

const LEAVE_SITE_CHOICES: &[BrowserChoiceSpec] = &[
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Cancel,
        labels: &["stay on page", "cancel", "stay"],
    },
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Leave,
        labels: &["leave page", "leave", "leave site"],
    },
];

const SAVE_PASSWORD_CHOICES: &[BrowserChoiceSpec] = &[
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Never,
        labels: &["don't save", "never", "not now"],
    },
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Save,
        labels: &["save", "remember"],
    },
];

const FIREFOX_PROMPTS: &[BrowserPromptSpec] = &[
    BrowserPromptSpec {
        prompt: BrowserOptionPrompt::LeaveSite,
        titles: &[
            "leave page",
            "leave site",
            "leave this page",
            "confirm that you want to leave",
        ],
        choices: LEAVE_SITE_CHOICES,
    },
    BrowserPromptSpec {
        prompt: BrowserOptionPrompt::SavePassword,
        titles: &[
            "remember password",
            "save password",
            "remember this password",
        ],
        choices: SAVE_PASSWORD_CHOICES,
    },
];

pub async fn choose(prompt: BrowserOptionPrompt, choice: BrowserOptionChoice) -> Result<String> {
    if prompt == BrowserOptionPrompt::LeaveSite && choice == BrowserOptionChoice::Cancel {
        let window = crate::firefox::window::find_firefox_window(None)?;
        let activation_note = crate::firefox::actions::context::activate_window_note(&window.id);
        crate::window::send_key(&window.id, "Escape")?;
        return Ok(format!(
            "firefox option leave-site cancel | dismissed prompt with Escape in window {} ({})",
            window.id, activation_note
        ));
    }

    if prompt == BrowserOptionPrompt::SavePassword && choice == BrowserOptionChoice::Never {
        let window = crate::firefox::window::find_firefox_window(None)?;
        let activation_note = crate::firefox::actions::context::activate_window_note(&window.id);
        crate::window::send_key(&window.id, "Escape")?;
        return Ok(format!(
            "firefox option save-password never | dismissed prompt with Escape in window {} ({})",
            window.id, activation_note
        ));
    }

    let window = crate::firefox::window::find_firefox_window(None)?;
    let profile = BrowserOptionProfile {
        browser_name: "firefox",
        query_candidates: vec![
            window.name.clone(),
            "mozilla firefox".to_string(),
            "firefox".to_string(),
        ],
        prompts: FIREFOX_PROMPTS,
        window_title: window.name,
    };
    let button = browser_options::choose(&profile, prompt, choice).await?;
    let label = button.line_label();
    let path = button.path.join(" > ");
    let click_summary =
        crate::chrome::actions::click::click_target_node(&button, &label, &path).await?;

    Ok(format!(
        "firefox option {} {} | {}",
        prompt.canonical_name(),
        choice.canonical_name(),
        click_summary
    ))
}
