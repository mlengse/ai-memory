# Design: CAS Preconditions & Section Patching (`specs/032-cas-memory-patch`)

Section-level, compare-and-swap updates for `procedural` / `semantic` memory
pages: an agent can rewrite one `## Heading` section without re-emitting the
whole document, and two parallel sessions cannot silently overwrite each other.

## Why

A long memory page holds many independent rules and decisions. Rewriting the
whole body to change one rule risks losing unrelated content and races other
sessions. Patching a section is a smaller write and a natural unit for
optimistic concurrency.

## Data model (V100)

Three tables, intentionally unscoped — a memory page is identified by its own
`page_id`, not a workspace/project path (the feature spec models it that way):

| Table | Purpose |
|-------|---------|
| `memory_pages` | One row per page: `id` (page_id), `revision_id` (current, bumps per patch), `page_type` (`procedural` \| `semantic`). |
| `memory_sections` | Ordered sections: `page_id`, `heading`, `section_order` (1-based), `content`, `content_hash` (SHA-256 hex). Section identity is `heading` + `section_order`; both are unique per page. |
| `revision` | Audit snapshots: `revision_id` (the revision the snapshot produced), `page_id`, `heading`, `section_order`, `old_content`, `created_at`. Written before a `replace` takes effect (FR-004). |

`content_hash` is defined once in `ai_memory_core::content_hash` (SHA-256,
lowercase hex), so the hash a reader sees and the hash a patch verifies are the
same function.

## Operations

The single store entry point is
`ai_memory_store::patch_memory_section(conn, page_id, operation, heading,
order, new_content, &preconditions, role)`. It handles all three operations
(FR-006):

| Operation | Behaviour |
|-----------|-----------|
| `replace` | Replace an existing section's content; the old content is snapshotted to `revision` first. |
| `append` | Append `new_content` to an existing section's content. |
| `add` | Append a brand-new section (next order); a duplicate heading is `invalid_operation`. |

Reading is `ai_memory_store::read_memory_page(conn, page_id)`, which returns the
heading list, per-section SHA-256, and current `page_id`/`revision_id` (FR-001).

## CAS preconditions (FR-002 / FR-003)

`CasPreconditions { expected_content_hash, expected_revision_id }` are both
optional. Every provided precondition must match the current state:

- `expected_revision_id` is checked against `memory_pages.revision_id` for every
  operation.
- `expected_content_hash` is checked against the target section's current hash
  for `replace` / `append`.

The first mismatch returns `StoreError::PreconditionFailed` and the transaction
rolls back — no write, no revision bump (FR-003). When both are omitted the
operation runs without CAS (spec assumption). This is exactly what makes two
parallel patches with the same preconditions yield one success and one
`precondition_failed` (US2, SC-002).

## Revision snapshot (FR-004 / US4)

Before a `replace` commits, `writer::record_revision_snapshot_in_tx` inserts the
old section content into `revision`, keyed by the new `revision_id` + `page_id`
+ `heading` + `section_order`. Querying `revision` by the returned
`revision_id`, `heading`, and `order` therefore recovers exactly what the patch
superseded. `add` / `append` record no snapshot (`revision_snapshot_recorded`
is `false`).

## Role-based access (FR-008)

`grants::MemoryRole` is `Editor` or `Viewer`. `project_authz::authorize_memory_patch`
is the gate every patch funnels through, checked before any read or write:
only `Editor` may patch; a `Viewer` gets `StoreError::Unauthorized` and nothing
is touched. Reads are unrestricted at this layer.

## Errors

| Error | Condition |
|-------|-----------|
| `precondition_failed` | Any provided precondition mismatches; no write. |
| `unauthorized` | Role is `viewer`; no write. |
| `section_not_found` | `replace`/`append` names a section that does not exist. |
| `invalid_operation` | `add` collides with an existing heading. |

## Testing

`ai-memory-store` carries the scenarios as unit tests in
`src/ops.rs::memory_patch_tests` (run with `cargo test -p ai-memory-store
memory_patch_tests`): single-section replace leaves other sections intact;
content-hash and revision-id mismatches are `precondition_failed` with no write;
exactly one of two conflicting patches wins; add/append behave; a replace
records a `revision` snapshot; a viewer is rejected. Migration compatibility is
covered by the existing migration suite (V100 is additive).

## Relationship to `memory_patch_page`

The MCP `memory_patch_page` tool edits a **wiki page** by path using
`page_patch::apply` (heading anchors + section hashes) and the wiki version
chain. This store-layer feature keeps memory pages in their own tables with an
explicit `revision_id` and a dedicated `revision` audit table. Both express the
same idea — patch one section under CAS — at different layers; the store entry
point above is the one a memory-page tool binding would call.
