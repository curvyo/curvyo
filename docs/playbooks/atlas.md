# Playbook: finding code with ProjectAtlas

Use ProjectAtlas before broad grep or read passes. Its tools are deferred: load
them first with ToolSearch `select:mcp__projectatlas__atlas_search,mcp__projectatlas__atlas_symbols,mcp__projectatlas__atlas_outline,mcp__projectatlas__atlas_slice,mcp__projectatlas__atlas_files`.
Pass `project_path=<absolute path of the checkout>` on every call. Run
`projectatlas init` once in the checkout (about 40 s). Run `projectatlas scan`
after every pull or branch switch and after a milestone; the index does not
follow edits. If an answer says `refresh_required`, run `projectatlas scan` or
`atlas_watch_once`. If a call fails, fall back to grep and read and carry on.
After a merge, `git switch main && git pull --ff-only`, then scan. A session
that was started before the ProjectAtlas version changed keeps its old server
process (`schema_version_mismatch` in the answers): it needs a restart.
