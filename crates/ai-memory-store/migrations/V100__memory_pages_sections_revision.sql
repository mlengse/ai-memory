-- Memory pages, their sections, and per-revision section snapshots
-- (specs/032-cas-memory-patch).
--
-- Distinct from the versioned wiki `pages` table: a memory page is patched
-- section-by-section, carries an explicit `revision_id` that bumps on every
-- successful patch, and keeps an audit snapshot of the content it replaced
-- (FR-004). These tables are intentionally unscoped — the feature spec models
-- a memory page by its own `page_id`, not by a workspace/project path.

CREATE TABLE memory_pages (
    id           BLOB PRIMARY KEY NOT NULL,          -- page_id (16-byte UUID)
    revision_id  BLOB NOT NULL,                      -- current revision (16-byte UUID)
    page_type    TEXT NOT NULL CHECK (page_type IN ('procedural','semantic')),
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE memory_sections (
    id            BLOB PRIMARY KEY NOT NULL,
    page_id       BLOB NOT NULL REFERENCES memory_pages(id) ON DELETE CASCADE,
    heading       TEXT NOT NULL,
    section_order INTEGER NOT NULL,                  -- 1-based; `order` is reserved
    content       TEXT NOT NULL,
    content_hash  TEXT NOT NULL,                     -- SHA-256 hex
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

-- Section identity = heading + order within the page (clarified Q2), and a
-- page's headings are unique so a patch naming a heading is unambiguous.
CREATE UNIQUE INDEX idx_memory_sections_page_order
    ON memory_sections(page_id, section_order);
CREATE UNIQUE INDEX idx_memory_sections_page_heading
    ON memory_sections(page_id, heading);

-- Snapshot of a section's content before a patch replaced it (FR-004), keyed
-- by the revision the snapshot belongs to, then page + section identity.
CREATE TABLE revision (
    id            BLOB PRIMARY KEY NOT NULL,
    revision_id   BLOB NOT NULL,
    page_id       BLOB NOT NULL REFERENCES memory_pages(id) ON DELETE CASCADE,
    heading       TEXT NOT NULL,
    section_order INTEGER NOT NULL,
    old_content   TEXT NOT NULL,
    created_at    INTEGER NOT NULL
);

CREATE INDEX idx_revision_page_revision
    ON revision(page_id, revision_id);
CREATE INDEX idx_revision_section
    ON revision(page_id, heading, section_order);
