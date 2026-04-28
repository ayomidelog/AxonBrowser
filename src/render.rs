use crate::model::{LiveNode, UiNode};

pub fn render_tree(root: &UiNode) -> String {
    let mut out = String::new();
    render_into(root, 0, &mut out);
    out
}

pub fn render_live_matches(matches: &[LiveNode]) -> String {
    let mut out = String::new();

    for (index, item) in matches.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }

        out.push_str(&format!("match {}\n", index + 1));
        out.push_str(&format!("  node: {}\n", item.line_label()));
        out.push_str("  path:\n");
        for segment in &item.path {
            out.push_str("    - ");
            out.push_str(segment);
            out.push('\n');
        }
    }

    if matches.is_empty() {
        out.push_str("no matches\n");
    }

    out
}

pub fn render_chrome_locator(locator: &str, node: &LiveNode) -> String {
    let mut out = String::new();
    out.push_str(&format!("locator: {}\n", locator));
    out.push_str(&render_live_node(node));
    out
}

pub fn render_live_node(node: &LiveNode) -> String {
    let mut out = String::new();
    out.push_str(&format!("node: {}\n", node.line_label()));
    out.push_str("path:\n");
    for segment in &node.path {
        out.push_str("  - ");
        out.push_str(segment);
        out.push('\n');
    }
    out
}

fn render_into(node: &UiNode, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    out.push_str(&indent);
    out.push_str(&node.line_label());
    out.push('\n');

    for child in &node.children {
        render_into(child, depth + 1, out);
    }
}
