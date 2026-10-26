-- Which revision of the page-derivation rules (frontmatter, title, links,
-- sources) the stored values were computed with. The server re-derives pages
-- below the current revision on start, so a rule change or a page that
-- predates this column needs no migration script.
ALTER TABLE pages ADD COLUMN derived_rev smallint NOT NULL DEFAULT 0;
