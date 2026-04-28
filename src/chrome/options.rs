use anyhow::Result;

use crate::browser_options::{
    self, BrowserChoiceSpec, BrowserOptionChoice, BrowserOptionProfile, BrowserOptionPrompt,
    BrowserPromptSpec,
};

const LEAVE_SITE_CHOICES: &[BrowserChoiceSpec] = &[
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Cancel,
        labels: &["cancel"],
    },
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Leave,
        labels: &["leave"],
    },
];

const SAVE_PASSWORD_CHOICES: &[BrowserChoiceSpec] = &[
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Never,
        labels: &["not now", "never"],
    },
    BrowserChoiceSpec {
        choice: BrowserOptionChoice::Save,
        labels: &["save"],
    },
];

const CHROME_PROMPTS: &[BrowserPromptSpec] = &[
    BrowserPromptSpec {
        prompt: BrowserOptionPrompt::LeaveSite,
        titles: &["leave site", "leave site?"],
        choices: LEAVE_SITE_CHOICES,
    },
    BrowserPromptSpec {
        prompt: BrowserOptionPrompt::SavePassword,
        titles: &[
            "save password",
            "save password?",
            "save your password",
            "save your password?",
        ],
        choices: SAVE_PASSWORD_CHOICES,
    },
];

pub async fn choose(prompt: BrowserOptionPrompt, choice: BrowserOptionChoice) -> Result<String> {
    let window = crate::chrome::window::find_browser_window(None)?;
    let profile = BrowserOptionProfile {
        browser_name: "chrome",
        query_candidates: vec![
            window.name.clone(),
            "chrome".to_string(),
            "chromium".to_string(),
        ],
        prompts: CHROME_PROMPTS,
        window_title: window.name,
    };
    let button = browser_options::choose(&profile, prompt, choice).await?;
    let label = button.line_label();
    let path = button.path.join(" > ");
    let click_summary =
        crate::chrome::actions::click::click_target_node(&button, &label, &path).await?;

    Ok(format!(
        "chrome option {} {} | {}",
        prompt.canonical_name(),
        choice.canonical_name(),
        click_summary
    ))
}
