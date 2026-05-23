use std::collections::HashSet;

use anyhow::{Context, Result, anyhow};
use async_recursion::async_recursion;
use atspi::{
    AccessibilityConnection, ObjectRefOwned,
    connection::{read_session_accessibility, set_session_accessibility},
    proxy::{
        CoordType,
        accessible::{AccessibleProxy, ObjectRefExt},
        component::ComponentProxy,
        proxy_ext::ProxyExt,
    },
};

use crate::{
    model::{LiveNode, UiNode, line_label},
    selector::Selector,
};

pub async fn resolve(query: &str, selectors: &[Selector]) -> Result<Vec<LiveNode>> {
    let root = resolve_root(query).await?;
    resolve_within_scopes(vec![root], selectors).await
}

pub async fn inspect_live(root: &LiveNode) -> Result<UiNode> {
    ensure_accessibility_enabled().await?;

    let connection = connect_accessibility().await?;
    let accessible = root
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind live node for tree inspection")?;

    let mut visited = HashSet::new();
    build_tree(&accessible, connection.connection(), &mut visited).await
}

pub async fn resolve_within_scope(
    scope: &LiveNode,
    selectors: &[Selector],
) -> Result<Vec<LiveNode>> {
    resolve_within_scopes(vec![scope.clone()], selectors).await
}

pub async fn descendants(scope: &LiveNode) -> Result<Vec<LiveNode>> {
    let connection = connect_accessibility().await?;

    let mut nodes = Vec::new();
    collect_descendants(scope, connection.connection(), &mut nodes).await?;
    Ok(nodes)
}

pub async fn clickable_point(node: &LiveNode) -> Result<(i32, i32)> {
    let component = bind_component(node).await?;
    read_clickable_point(&component).await
}

pub async fn component_extents(node: &LiveNode) -> Result<(i32, i32, i32, i32)> {
    let component = bind_component(node).await?;
    component
        .get_extents(CoordType::Screen)
        .await
        .context("failed to read component extents")
}

pub async fn read_state_set(node: &LiveNode) -> Result<atspi::StateSet> {
    let connection = connect_accessibility().await?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for state lookup")?;
    accessible
        .get_state()
        .await
        .context("failed to read AT-SPI state set for matched node")
}

async fn resolve_root(query: &str) -> Result<LiveNode> {
    ensure_accessibility_enabled().await?;

    let connection = connect_accessibility().await?;

    let registry = connection
        .root_accessible_on_registry()
        .await
        .context("failed to get the AT-SPI registry root")?;

    let applications = registry
        .get_children()
        .await
        .context("failed to list desktop applications from the AT-SPI registry")?;

    let needle = normalize_query(query)?;

    for app_ref in applications {
        if app_ref.is_null() {
            continue;
        }

        let app = app_ref
            .as_accessible_proxy(connection.connection())
            .await
            .with_context(|| format!("failed to bind app proxy for {}", debug_ref(&app_ref)))?;

        if let Some(found) = find_matching_live_node(&app, &needle, connection.connection()).await?
        {
            return Ok(found);
        }
    }

    Err(anyhow!(
        "no accessible application or window matched query {:?}",
        query
    ))
}

async fn resolve_within_scopes(
    mut scopes: Vec<LiveNode>,
    selectors: &[Selector],
) -> Result<Vec<LiveNode>> {
    if selectors.is_empty() {
        return Ok(scopes);
    }

    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;

    for selector in selectors {
        let mut next = Vec::new();
        for scope in &scopes {
            collect_descendant_matches(scope, selector, connection.connection(), &mut next).await?;
        }
        scopes = next;
        if scopes.is_empty() {
            break;
        }
    }

    Ok(scopes)
}

async fn ensure_accessibility_enabled() -> Result<()> {
    let enabled = read_session_accessibility()
        .await
        .context("failed to read AT-SPI IsEnabled status from the session bus")?;

    if !enabled {
        set_session_accessibility(true)
            .await
            .context("failed to enable AT-SPI accessibility on the session bus")?;
    }

    Ok(())
}

async fn connect_accessibility() -> Result<AccessibilityConnection> {
    match AccessibilityConnection::new().await {
        Ok(connection) => Ok(connection),
        Err(_) => {
            crate::runtime::repair_accessibility_stack()
                .context("failed to repair the AT-SPI accessibility stack")?;
            AccessibilityConnection::new()
                .await
                .context("failed to connect to the AT-SPI accessibility bus")
        }
    }
}

#[async_recursion]
async fn find_matching_live_node(
    node: &AccessibleProxy<'_>,
    needle: &str,
    conn: &atspi::zbus::Connection,
) -> Result<Option<LiveNode>> {
    let role = read_role(node).await;
    let name = read_name(node).await;
    let label = line_label(&role, name.as_deref());

    if name
        .as_deref()
        .is_some_and(|value| matches_query(value, needle))
    {
        return Ok(Some(LiveNode {
            object_ref: ObjectRefOwned::try_from(node)
                .context("failed to extract object ref for matched root")?,
            role,
            name,
            path: vec![label],
        }));
    }

    let children = node
        .get_children()
        .await
        .with_context(|| format!("failed to read children for {}", label))?;

    for child_ref in children {
        if child_ref.is_null() {
            continue;
        }

        let child = child_ref
            .as_accessible_proxy(conn)
            .await
            .with_context(|| format!("failed to bind child proxy for {}", debug_ref(&child_ref)))?;

        if let Some(found) = find_matching_live_node(&child, needle, conn).await? {
            return Ok(Some(found));
        }
    }

    Ok(None)
}

#[async_recursion]
async fn collect_descendant_matches(
    scope: &LiveNode,
    selector: &Selector,
    conn: &atspi::zbus::Connection,
    matches: &mut Vec<LiveNode>,
) -> Result<()> {
    let accessible = scope
        .object_ref
        .as_accessible_proxy(conn)
        .await
        .with_context(|| {
            format!(
                "failed to bind scope proxy while traversing descendants for {}",
                scope.line_label()
            )
        })?;

    let children = accessible
        .get_children()
        .await
        .with_context(|| format!("failed to read children for {}", scope.line_label()))?;

    for child_ref in children {
        if child_ref.is_null() {
            continue;
        }

        let child = child_ref
            .as_accessible_proxy(conn)
            .await
            .with_context(|| format!("failed to bind child proxy for {}", debug_ref(&child_ref)))?;

        let role = read_role(&child).await;
        let name = read_name(&child).await;
        let label = line_label(&role, name.as_deref());
        let path = extend_path(&scope.path, &label);

        let live = LiveNode {
            object_ref: child_ref.clone(),
            role,
            name,
            path: path.clone(),
        };

        if selector.matches_live(&live) {
            matches.push(live.clone());
        }

        collect_descendant_matches(&live, selector, conn, matches).await?;
    }

    Ok(())
}

#[async_recursion]
async fn collect_descendants(
    scope: &LiveNode,
    conn: &atspi::zbus::Connection,
    descendants: &mut Vec<LiveNode>,
) -> Result<()> {
    let accessible = scope
        .object_ref
        .as_accessible_proxy(conn)
        .await
        .with_context(|| {
            format!(
                "failed to bind scope proxy while traversing descendants for {}",
                scope.line_label()
            )
        })?;

    let children = accessible
        .get_children()
        .await
        .with_context(|| format!("failed to read children for {}", scope.line_label()))?;

    for child_ref in children {
        if child_ref.is_null() {
            continue;
        }

        let child = child_ref
            .as_accessible_proxy(conn)
            .await
            .with_context(|| format!("failed to bind child proxy for {}", debug_ref(&child_ref)))?;

        let role = read_role(&child).await;
        let name = read_name(&child).await;
        let label = line_label(&role, name.as_deref());
        let path = extend_path(&scope.path, &label);

        let live = LiveNode {
            object_ref: child_ref.clone(),
            role,
            name,
            path,
        };

        descendants.push(live.clone());
        collect_descendants(&live, conn, descendants).await?;
    }

    Ok(())
}

#[async_recursion]
async fn build_tree(
    node: &AccessibleProxy<'_>,
    conn: &atspi::zbus::Connection,
    visited: &mut HashSet<String>,
) -> Result<UiNode> {
    let key = node_key(node);
    if !visited.insert(key) {
        let role = read_role(node).await;
        let name = read_name(node).await;
        return Ok(UiNode::new(role, name, Vec::new()));
    }

    let role = read_role(node).await;
    let name = read_name(node).await;
    let children_refs = node.get_children().await.with_context(|| {
        format!(
            "failed to read children for {}",
            line_label(&role, name.as_deref())
        )
    })?;

    let mut children = Vec::new();
    for child_ref in children_refs {
        if child_ref.is_null() {
            continue;
        }

        let child = child_ref
            .as_accessible_proxy(conn)
            .await
            .with_context(|| format!("failed to bind child proxy for {}", debug_ref(&child_ref)))?;
        children.push(build_tree(&child, conn, visited).await?);
    }

    Ok(UiNode::new(role, name, children))
}

async fn read_clickable_point(component: &ComponentProxy<'_>) -> Result<(i32, i32)> {
    let (x, y, width, height) = component
        .get_extents(CoordType::Screen)
        .await
        .context("failed to read component extents")?;

    if width <= 0 || height <= 0 {
        return Err(anyhow!("matched node has non-visible extents"));
    }

    Ok((x + (width / 2), y + (height / 2)))
}

async fn read_role(node: &AccessibleProxy<'_>) -> String {
    node.get_role_name()
        .await
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(title_case_role)
        .unwrap_or_else(|| "Unknown".to_string())
}

async fn read_name(node: &AccessibleProxy<'_>) -> Option<String> {
    node.name()
        .await
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn matches_query(name: &str, needle: &str) -> bool {
    normalize_for_match(name).contains(needle)
}

fn normalize_query(query: &str) -> Result<String> {
    let needle = normalize_for_match(query);
    if needle.is_empty() {
        return Err(anyhow!("query must not be empty"));
    }
    Ok(needle)
}

async fn bind_component(node: &LiveNode) -> Result<ComponentProxy<'_>> {
    let connection = AccessibilityConnection::new()
        .await
        .context("failed to connect to the AT-SPI accessibility bus")?;
    let accessible = node
        .object_ref
        .as_accessible_proxy(connection.connection())
        .await
        .context("failed to bind matched node for component lookup")?;
    let proxies = accessible
        .proxies()
        .await
        .context("failed to inspect matched node interfaces")?;
    proxies
        .component()
        .await
        .context("matched node does not expose Component interface")
}

fn normalize_for_match(input: &str) -> String {
    strip_invisible_format_chars(input)
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_invisible_format_chars(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !matches!(ch, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'))
        .collect()
}

fn title_case_role(role: String) -> String {
    role.split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extend_path(path: &[String], label: &str) -> Vec<String> {
    let mut next = path.to_vec();
    next.push(label.to_string());
    next
}

fn node_key(node: &AccessibleProxy<'_>) -> String {
    match ObjectRefOwned::try_from(node) {
        Ok(object_ref) => debug_ref(&object_ref),
        Err(_) => "<unknown-object-ref>".to_string(),
    }
}

fn debug_ref(node: &ObjectRefOwned) -> String {
    format!(
        "{} {}",
        node.name_as_str().unwrap_or("<no-name>"),
        node.path_as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::normalize_for_match;

    #[test]
    fn strips_zero_width_characters_for_matching() {
        let with_hidden = "Microsoft\u{200b} Edge";
        assert_eq!(normalize_for_match(with_hidden), "microsoft edge");
    }
}
