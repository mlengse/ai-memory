//! `ai-memory sulur-patch` — patch a memory page section (replace/add/append) with optional CAS preconditions.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::cli::{Config, SulurPatchArgs};
use crate::http_client::{ServerEndpoint, post_json};

/// Request body for the `sulur_patch` endpoint.
#[derive(Serialize)]
struct SulurPatchRequest {
    workspace: String,
    project: String,
    page_id: String,
    section_heading: String,
    section_order: u32,
    operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_content_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision_id: Option<String>,
    actor_role: String,
}

/// Response from the `sulur_patch` endpoint.
#[derive(Debug, Deserialize, Serialize)]
struct SulurPatchResponse {
    page_id: String,
    revision_id: String,
    section_heading: String,
    section_order: u32,
    content_hash: String,
    operation: String,
    revision_snapshot_recorded: bool,
}

/// Run the `sulur-patch` subcommand.
///
/// # Errors
/// Returns an error if the server is unreachable, returns non-2xx, or required arguments are missing/invalid.
pub async fn run(config: &Config, args: SulurPatchArgs) -> Result<()> {
    let i18n = &config.i18n;
    super::refuse_empty_explicit(args.workspace.as_deref(), "--workspace")?;
    super::refuse_empty_explicit(args.project.as_deref(), "--project")?;

    let page_id = args.page_id.trim();
    if page_id.is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-page-id-was-passed-empty-specify-the-memory-page-id")
        );
    }
    if args.section_heading.trim().is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-section-heading-was-passed-empty-specify-the-section-heading")
        );
    }
    if args.operation.trim().is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-operation-was-passed-empty-specify-replace-add-or-append")
        );
    }
    if args.actor_role.trim().is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-actor-role-was-passed-empty-specify-editor-or-viewer")
        );
    }

    // Validate operation
    let op = args.operation.to_lowercase();
    if !["replace", "add", "append"].contains(&op.as_str()) {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-invalid-operation-must-be-replace-add-or-append")
        );
    }

    // Validate actor_role
    let role = args.actor_role.to_lowercase();
    if !["editor", "viewer"].contains(&role.as_str()) {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-invalid-actor-role-must-be-editor-or-viewer")
        );
    }

    // new_content is required for replace and add
    if (op == "replace" || op == "add") && args.new_content.as_deref().unwrap_or("").trim().is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-patch-new-content-required-for-replace-and-add")
        );
    }

    let (workspace, project) =
        super::resolve_scope(config, args.workspace.as_deref(), args.project.as_deref())?;
    let ep = ServerEndpoint::from_config_resolving_auth(config).await;

    let request = SulurPatchRequest {
        workspace: workspace.clone(),
        project: project.clone(),
        page_id: page_id.to_string(),
        section_heading: args.section_heading.clone(),
        section_order: args.section_order,
        operation: op,
        new_content: args.new_content.clone().filter(|s| !s.trim().is_empty()),
        expected_content_hash: args.expected_content_hash.clone().filter(|s| !s.trim().is_empty()),
        expected_revision_id: args.expected_revision_id.clone().filter(|s| !s.trim().is_empty()),
        actor_role: role,
    };

    let resp: SulurPatchResponse = post_json(&ep, "/admin/sulur-patch", &request).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        print!("{}", render_text(&config.i18n, &resp));
    }
    Ok(())
}

fn render_text(i18n: &ai_memory_core::i18n::Localizer, resp: &SulurPatchResponse) -> String {
    let mut out = String::new();
    out.push_str(&i18n.format(
        "cli-sulur-patch-page-id",
        &[("value0", terminal_text(&resp.page_id))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-revision-id",
        &[("value0", terminal_text(&resp.revision_id))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-section-heading",
        &[("value0", terminal_text(&resp.section_heading))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-section-order",
        &[("value0", resp.section_order.to_string())],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-content-hash",
        &[("value0", terminal_text(&resp.content_hash))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-operation",
        &[("value0", terminal_text(&resp.operation))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-patch-revision-snapshot-recorded",
        &[("value0", if resp.revision_snapshot_recorded { "true" } else { "false" })],
    ));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_output_shows_all_fields() {
        let resp = SulurPatchResponse {
            page_id: "01a16f89-8ca9-7f40-8847-eb44e095e3ea".into(),
            revision_id: "01a16f89-eb81-70a2-8ba4-c84c5783b7fb".into(),
            section_heading: "Rules".into(),
            section_order: 1,
            content_hash: "abc123".into(),
            operation: "replace".into(),
            revision_snapshot_recorded: true,
        };
        let output = render_text(&ai_memory_core::i18n::Localizer::default(), &resp);
        assert!(output.contains("page-id: 01a16f89-8ca9-7f40-8847-eb44e095e3ea"));
        assert!(output.contains("revision-id: 01a16f89-eb81-70a2-8ba4-c84c5783b7fb"));
        assert!(output.contains("section-heading: Rules"));
        assert!(output.contains("section-order: 1"));
        assert!(output.contains("content-hash: abc123"));
        assert!(output.contains("operation: replace"));
        assert!(output.contains("revision-snapshot-recorded: true"));
    }
}
