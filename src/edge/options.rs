use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

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

const EDGE_PROMPTS: &[BrowserPromptSpec] = &[
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
    if prompt == BrowserOptionPrompt::LeaveSite && choice == BrowserOptionChoice::Cancel {
        let window = crate::edge::window::find_edge_window(None)?;
        let activation_note = crate::edge::actions::context::activate_window_note(&window.id);
        crate::window::send_key(&window.id, "Escape")?;
        return Ok(format!(
            "edge option leave-site cancel | dismissed prompt with Escape in window {} ({})",
            window.id, activation_note
        ));
    }

    let window = crate::edge::window::find_edge_window(None)?;
    let profile = BrowserOptionProfile {
        browser_name: "edge",
        query_candidates: vec![
            window.name.clone(),
            "microsoft edge".to_string(),
            "msedge".to_string(),
            "edge".to_string(),
        ],
        prompts: EDGE_PROMPTS,
        window_title: window.name.clone(),
    };
    let button = match tokio::time::timeout(
        Duration::from_millis(2500),
        browser_options::choose(&profile, prompt, choice),
    )
    .await
    {
        Ok(Ok(button)) => button,
        Ok(Err(_)) if prompt == BrowserOptionPrompt::SavePassword => {
            return save_password_keyboard_fallback(&window, choice).await;
        }
        Err(_) if prompt == BrowserOptionPrompt::SavePassword => {
            return save_password_keyboard_fallback(&window, choice).await;
        }
        Ok(Err(err)) => return Err(err),
        Err(err) => return Err(err.into()),
    };
    let label = button.line_label();
    let path = button.path.join(" > ");
    let click_summary =
        crate::chrome::actions::click::click_target_node(&button, &label, &path).await?;

    Ok(format!(
        "edge option {} {} | {}",
        prompt.canonical_name(),
        choice.canonical_name(),
        click_summary
    ))
}

async fn save_password_keyboard_fallback(
    window: &crate::window::WindowMatch,
    choice: BrowserOptionChoice,
) -> Result<String> {
    let activation_note = crate::edge::actions::context::activate_window_note(&window.id);
    let prompt_x = i32::try_from(window.width.saturating_sub(425)).unwrap_or(855);
    crate::window::mousemove_click(&window.id, prompt_x, 276)?;
    sleep(Duration::from_millis(150)).await;
    crate::window::send_key(&window.id, "Tab")?;
    crate::window::send_key(&window.id, "Tab")?;
    if choice == BrowserOptionChoice::Never {
        crate::window::send_key(&window.id, "Tab")?;
    }
    crate::window::send_key(&window.id, "Return")?;

    Ok(format!(
        "edge option save-password {} | triggered prompt action via keyboard fallback in window {} ({})",
        choice.canonical_name(),
        window.id,
        activation_note
    ))
}
