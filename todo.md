# bldhnd — Roadmap to 1000%

Everything needed to go from "works on my machine" to a Lidarr/Sonarr/Radarr
replacement that's genuinely nice to live in. Ordered by impact; check things off.

## Phase 1 — Core reliability (must not embarrass us)

- [ ] **Download progress in the UI.** `Request::DownloadProgress` IPC exists but is never polled. Poll active downloads every ~1s while the Downloads tab is visible; show %, transferred/total, and speed. Without this, queued downloads look frozen even when working.
- [ ] **Auto-refresh Downloads tab** on an interval (or on state-change notifications) instead of manual `r`.
- [ ] **Resume queue on service restart.** Downloads persisted in redb with state `queued` are never picked back up at boot. On startup, re-drive all non-terminal downloads through their backend.
- [ ] **Retry failed downloads.** `r`-style retry key on a failed row (re-resolve uri → backend again). Failures today are dead ends.
- [ ] **Long-lived torrent engine.** Each torrent currently gets a throwaway librqbit session: no resume-after-restart, no cross-download DHT sharing, no seeding. Give the service one persistent session thread + handle registry.
- [ ] **End-to-end verify soulseek download path** (search hit → StartDownload with username/filename → file lands in download_dir). It's wired but has never been exercised against a real slsk server.
- [ ] **Server redeploy + ingest verification.** `bldhnd.fargone.sh` was 404ing; confirm `/music`, `/media`, `/stats` respond, and spot-check MB release counts (`stats`) after the ranking-fix deploy.

## Phase 2 — Search that feels great

- [ ] **Pagination / scrollback.** Server supports page param (`p`, 50/page); TUI shows only the first 50 with no way to load more. Add PgDn/n "next page" or infinite scroll.
- [ ] **Backend results detail columns.** Backend hits carry seeders/peers (torrent), bitrate/duration/free-slot (soulseek) — dropped on the floor by the IPC mapping. Show them; let people pick the good rip, not the first row.
- [ ] **Choose download backend per search.** Music→soulseek, Movie/Series→torrent is hardcoded in `search_backend()`. Add a backend selector (or try-all-and-merge).
- [ ] **Back navigation.** After Enter drills KB→backend results there's no way back to the KB list except re-searching. Esc/B should pop the stack.
- [ ] **Search history** (up-arrow in empty query input recalls previous queries).
- [ ] **Debounce/live search** as you type (300ms idle), so Enter becomes optional.
- [ ] **Result count + timing polish**: show "page 1/12", total known matches if the server can report them.

## Phase 3 — Library management

- [ ] **Browse actual files per volume** (the point of a library!). Tree/list of artists/albums under each volume path, not just usage stats.
- [ ] **Surface volume priority editing** — schema has it, UI doesn't. Drag-free: `-`/`+` keys on a volume row.
- [ ] **Destination preview before queueing**: which volume will this land in and why (priority/cap logic), shown on the download confirm.
- [ ] **Post-download organization**: rename/move into `Artist/Album (Year)/` layout per media type (this is 80% of what *arr apps actually do).
- [ ] **Delete from library** with confirm (files + db row).
- [ ] **Library scan feedback**: `s` scans silently — stream progress ("scanned 1,204 files…") into the status line.

## Phase 4 — Settings completeness

- [ ] **Quality settings UI** (`QualitySettings`: max size, min bitrate, preferred languages/formats) — fields exist in config, no UI, nothing enforces them. Wire UI + enforcement into backend-search result filtering/ranking.
- [ ] **Notifications UI + webhook delivery** (`NotificationSettings.webhook_url` stored but never sent).
- [ ] **Plex UI + end-to-end test** (url/token/auto_scan; `PlexClient.process_download` runs on completion — verify against a real server).
- [ ] **Test-connection buttons** for soulseek/torrent indexer/SABnzbd/Plex rows ("✓ connected in 230ms" inline).
- [ ] **Import/export config** as JSON for backup/migration.

## Phase 5 — Delight

- [ ] **Help overlay** (`?`): full keymap for the focused tab. The footer hint line can't carry everything.
- [ ] **Subtitles**: `SubtitleBackend` (opensubtitles) exists in libs/download, completely unwired. Search + attach on movie/series completion.
- [ ] **Playback**: `p` on a completed album/file opens mpv (or user-configured player) on the path.
- [ ] **Mouse everywhere**: results click-to-select + double-click-to-act; downloads click-select; settings section click-nav.
- [ ] **Theme support** (config-driven palette; at minimum dark/light toggle).
- [ ] **Stats on Home**: KB sizes via server `/stats` endpoint (exists!), local library totals, queue health — make Home a real dashboard.
- [ ] **Toast/notification area** instead of status-line-only errors (transient, auto-dismissing, colored).
- [ ] **Logs filtering** in Logs tab (level filter, text search, follow-toggle).

## Known rough edges (fix opportunistically)

- [ ] `service/src/web.rs` is a stub (`start_web_server` does nothing) — either implement the HTTP API or delete it.
- [ ] `users.rs` multi-user support exists but nothing exposes auth; decide: single-user forever, or wire it.
- [ ] `AppTab` enum in views/mod.rs is dead code; indices are hardcoded ints everywhere — consolidate.
- [ ] bobatea warnings (unused fields/imports) — upstream cleanup pass.
- [ ] `download` crate warnings (dead fields: torrent_link/info_hash unused reads etc.) — prune or use.
- [ ] Torrent downloads ignore `progress_callback` between Connecting and Complete — librqbit exposes per-torrent stats; feed them through once Phase 1 progress polling lands.
- [ ] WikiData ingest: sample-check film/TV records for label quality after next full ingest run.

## Definition of done

A stranger can: install, add their music volume, search "billy jean", watch it
download with live progress, find it organized under `Artist/Album/`, and get a
notification — without reading the source.
