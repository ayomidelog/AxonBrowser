use atspi::ObjectRefOwned;

#[derive(Debug, Clone)]
pub struct UiNode {
    pub role: String,
    pub name: Option<String>,
    pub children: Vec<UiNode>,
}

impl UiNode {
    pub fn new(role: impl Into<String>, name: Option<String>, children: Vec<UiNode>) -> Self {
        Self {
            role: role.into(),
            name,
            children,
        }
    }

    pub fn line_label(&self) -> String {
        line_label(&self.role, self.name.as_deref())
    }
}

#[derive(Debug, Clone)]
pub struct LiveNode {
    pub object_ref: ObjectRefOwned,
    pub role: String,
    pub name: Option<String>,
    pub path: Vec<String>,
}

impl LiveNode {
    pub fn line_label(&self) -> String {
        line_label(&self.role, self.name.as_deref())
    }
}

pub fn line_label(role: &str, name: Option<&str>) -> String {
    match name.filter(|value| !value.is_empty()) {
        Some(name) => format!("{}: \"{}\"", role, name),
        None => role.to_string(),
    }
}
