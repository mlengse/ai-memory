//! `ai-memory sulur-read-memory` — read a memory page by page_id via the store surface.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::show::{terminal_body, terminal_text};
use crate::cli::{Config, SulurReadMemoryArgs};
use crate::http_client::{ServerEndpoint, get_json};

/// Mirrors the server response for `sulur_read_memory`.
#[derive(Debug, Deserialize, Serialize)]
struct MemoryPageResponse {
    page_id: String,
    revision_id: String,
    page_type: String,
    sections: Vec<MemorySectionResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MemorySectionResponse {
    heading: String,
    order: u32,
    content_hash: String,
}

/// Run the `sulur-read-memory` subcommand.
///
/// # Errors
/// Returns an error if the server is unreachable, returns non-2xx, or the page_id is invalid.
pub async fn run(config: &Config, args: SulurReadMemoryArgs) -> Result<()> {
    let i18n = &config.i18n;
    super::refuse_empty_explicit(args.workspace.as_deref(), "--workspace")?;
    super::refuse_empty_explicit(args.project.as_deref(), "--project")?;

    let page_id = args.page_id.trim();
    if page_id.is_empty() {
        bail!(
            "{}",
            i18n.text("cli-sulur-read-memory-page-id-was-passed-empty-specify-the-memory-page-id")
        );
    }

    let (workspace, project) =
        super::resolve_scope(config, args.workspace.as_deref(), args.project.as_deref())?;
    let ep = ServerEndpoint::from_config_resolving_auth(config).await;

    let mut query_params = vec![
        ("workspace", workspace.as_str()),
        ("project", project.as_str()),
        ("page_id", page_id),
    ];
    if let Some(project_id) = args.project_id.as_deref() {
        query_params.push(("project_id", project_id));
    }

    let page: MemoryPageResponse = get_json(&ep, "/admin/sulur-read-memory", &query_params).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&page)?);
    } else {
        print!("{}", render_text(&config.i18n, &page));
    }
    Ok(())
}

fn render_text(i18n: &ai_memory_core::i18n::Localizer, page: &MemoryPageResponse) -> String {
    let mut out = String::new();
    out.push_str(&i18n.format(
        "cli-sulur-read-memory-page-id",
        &[("value0", terminal_text(&page.page_id))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-read-memory-revision-id",
        &[("value0", terminal_text(&page.revision_id))],
    ));
    out.push('\n');
    out.push_str(&i18n.format(
        "cli-sulur-read-memory-page-type",
        &[("value0", terminal_text(&page.page_type))],
    ));
    out.push('\n');
    out.push('\n');

    if page.sections.is_empty() {
        out.push_str(&i18n.text("cli-sulur-read-memory-no-sections"));
        out.push('\n');
    } else {
        out.push_str(&i18n.text("cli-sulur-read-memory-sections"));
        out.push('\n');
        for section in &page.sections {
            out.push_str(&format!(
                "  {} (order {}): hash={}\n",
                terminal_text(&section.heading),
                section.order,
                terminal_text(&section.content_hash)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(sections: Vec<MemorySectionResponse>) -> MemoryPageResponse {
        MemoryPageResponse {
            page_id: "01a16f89-8ca9-7f40-8847-eb44e095e3ea".into(),
            revision_id: "01a16f89-eb81-70a2-8ba4-c84c5783b7fb".into(),
            page_type: "semantic".into(),
            sections,
        }
    }

    #[test]
    fn text_output_shows_page_info_and_sections() {
        let p = page(vec![
            MemorySectionResponse {
                heading: "Rules".into(),
                order: 1,
                content_hash: "abc123".into(),
            },
            MemorySectionResponse {
                heading: "Decisions".into(),
                order: 2,
                content_hash: "def456".into(),
            },
        ]);
        let output = render_text(&ai_memory_core::i18n::Localizer::default(), &p);
        assert!(output.contains("page-id: 01a16f89-8ca9-7f40-8847-eb44e095e3ea"));
        assert!(output.contains("revision-id: 01a16f89-eb81-70a2-8ba4-c84c5783b7fb"));
        assert!(output.contains("page-type: semantic"));
        assert!(output.contains("Rules (order 1): hash=abc123"));
        assert!(output.contains("Decisions (order 2): hash=def456"));
    }

    #[test]
    fn text_output_shows_no_sections_message() {
        let p = page(vec![]);
        let output = render_text(&ai_memory_core::i18n::Localizer::default(), &p);
        assert!(output.contains("no sections"));
    }
}
