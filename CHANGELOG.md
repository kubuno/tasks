# Changelog

All notable changes to **kubuno-tasks** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this
project adheres to [Semantic Versioning](https://semver.org/). Entries are added under
`[Unreleased]` **as the change is made**; `_tools/release.sh` stamps them under the version
number at release time, and CI publishes that section as the GitHub Release notes.

## [Unreleased]

### Security

- **Security fixes from the shared database layer (kubuno-db 0.9.0).** The
  database password can no longer appear in a log through the debug output of
  the database settings.

### Changed

- **The module now runs on PostgreSQL, MySQL/MariaDB or SQLite.** The engine is
  a configuration choice (`database.engine`), read when the module starts; the
  same build talks to whichever the administrator picked. Nothing changes for an
  existing PostgreSQL instance — its data and its already-applied migrations are
  untouched — while a new instance can be stood up on MySQL/MariaDB or on a
  single SQLite file with no separate database server. Boards, columns, tasks,
  sub-tasks, labels, comments, attachments, reminders, retention, the CalDAV
  VTODO endpoint, iCalendar import/export and the local-first delta sync all
  behave identically on the three engines.

### Fixed

- **Local-first sync keeps working on every write path.** The change journal
  (the monotonic version each board and task carries, and the tombstone left
  when one is deleted) used to live in database triggers that only PostgreSQL
  has. It is now driven by the module itself at each write, so a sync client
  sees the same stream of changes whichever engine backs the instance —
  including bulk paths a trigger used to cover silently, such as a completed-task
  retention purge or the cascade when a whole board is deleted.

- **Database driver updated past an unfixable advisory.** The previous line
  pulled in an RSA implementation vulnerable to a timing side-channel
  (RUSTSEC-2023-0071) for which no fix will ever exist. The new line does not
  depend on it at all, and it refuses any SQL string built at run time unless it
  has been audited — the queries here were checked and marked.
- **Input validation library updated.** The version in use carried
  RUSTSEC-2024-0421 through its domain-name parser, which accepted Punycode
  labels that decode to plain ASCII — a mismatch an attacker can use to make two
  different names look like one.

## [0.1.8] - 2026-09-18

### Security

- **HTTP/2 layer updated to a patched release.** `h2` moves from 0.4.15 to
  0.4.19, closing a denial of service through unbounded empty DATA frames
  (RUSTSEC-2026-0258).
- **Error library updated to a patched release.** `anyhow` moves from 1.0.103
  to 1.0.104, closing an unsoundness in `Error::downcast_mut()`
  (RUSTSEC-2026-0190).
- **TLS library updated to a patched release.** The pinned `rustls` carried
  RUSTSEC-2026-0285 (medium). Every outbound HTTPS connection goes through it.

## [0.1.7] - 2026-09-18

### Added

- **A task can carry the instance's labels too, from its own panel.** It already
  had its board's labels — a vocabulary that belongs to the board and means
  something to whoever works on it. These are the other kind: the labels that
  also go on a file, a note or an event, owned by you and browsable across
  modules from one place. Neither replaces the other, so each has its own line
  and its own name.


### Changed



- **This module now installs as a Kubuno package (`.kbpkg`) only.** Its system
  packages (Debian/RPM and the Windows and macOS installers) are no longer
  built: the module is distributed as one `.kbpkg` per platform (Linux, Windows,
  macOS) that the Kubuno server installs itself — from the admin console, or
  offline with `kubuno modules:install <file>.kbpkg`.
- **Dates are formatted by the platform now, not by a library.** `date-fns` is
  gone from this module: the shared SDK exposes helpers built on `Intl`, which is
  localised for every language we ship and needs no locale bundle loaded. Call
  sites say what a date is FOR — `formatDate(d, 'date')` — and the platform
  decides how to write it, so a reader in Japanese no longer gets a French
  layout. Machine formats (keys, `<input type="date">` values) go through
  `toISODate` and friends, built from local calendar fields so the day cannot
  shift near midnight.
- **The README now opens with the module's logo.** The public README on
  GitHub now shows the module's designer logo (the same PNG shown as the
  browser tab icon and in the applications menu) at the top of the page — the
  repository landing now matches the icon a signed-in user sees inside the
  platform. The image ships in-repo, under `.github/logo.png`, so it renders
  even when the repo is browsed offline.

- **New Tasks logo** — a green hexagon with a white checkmark inside a circle,
  used as the browser-tab icon and in the applications menu. It replaces the
  generic check-square icon and is raster (PNG) designer artwork; the Tasks
  tab now has an icon of its own.

### Added

- **A lists overview, the new home of Tasks.** Opening Tasks now shows every
  list side by side, each in its own card: a heading, an "Add a task" row, the
  tasks themselves, and a foldable "Completed" section. Overdue tasks rise to
  the top of a hand-ordered list under a "Past" heading. Columns can be
  reordered by dragging the grab handle above a card, and a task can be dragged
  from one list to another. Each list card carries a menu offering how to sort
  it (my order, date created, date, starred recently, title), rename, delete,
  move to first position, hide, print, delete completed tasks, and clean up old
  ones; the sort and the folded state are remembered per person, so two people
  sharing a list can each order it their own way.
- **The sidebar now manages the lists.** A "Lists" section shows each list with
  a checkbox that adds or removes its column from the overview (it hides the
  column, it never touches the data), a count of the tasks left to do, and a
  shortcut to open a board-style list in its columns. "Create new list" adds
  one; the date-driven views (today, upcoming, overdue, important, completed)
  move into a foldable "Filters" section below.
- **Starred tasks.** A task can be starred from its row or its menu, and the new
  "Starred" view gathers the starred tasks of every list into one wide card,
  split into "recently" and "over a month ago", each row showing which list it
  came from. Starring is independent of priority, so the two no longer collide.
- **An in-place task composer.** "Add a task" opens an editor right where the
  task will live: a title, a details line, quick "Today"/"Tomorrow" chips, a
  full date-and-time picker, and a repeat control. It stays open so several
  tasks can be typed one after another, and a half-typed task is saved rather
  than lost when the editor closes.
- **A "Contains the words" field in the search filter panel, synced with the
  search bar both ways.** Opening the panel pre-fills the field with the bar's
  current text, and editing it rewrites the bar's text live (running the
  search as you type, exactly like typing in the bar). Tasks' search is plain
  free text — the status dropdown is a state filter with no text
  representation, so it intentionally stays panel-only. "Reset" now also
  clears the search bar's text.




### Fixed


- **A withdrawn dependency is no longer used.** A crate deep in the tree
  (`spin` 0.9.8, pulled in through the HTTP stack) was yanked by its authors.
  No vulnerability was announced, but a withdrawn crate has no business in a
  release; the lockfile now takes the version that replaced it.
- **The package could not be built where `zip` is absent.** The Windows job of
  the continuous integration has no `zip`, so the Windows package was simply lost
  the first time it was attempted — a script failure, not a build failure. The
  builder now falls back to 7-Zip, then to PowerShell.
### Added

- **This module now ships a `.kbpkg`** — the single package format a Kubuno
  server installs by itself, the same file on Linux, Windows and macOS. It
  carries the same binary, interface and manifest as the system packages,
  arranged the way the server expects to find a module on disk, plus a
  `SHA256SUMS` so a copy carried offline can be checked without the catalogue.
  Nothing changes for existing installations: the `.deb`, `.rpm`, `.exe` and
  `.pkg` are still published, and a catalogue that sees both simply prefers the
  new one. It is also the only format the server can unpack without an external
  tool, which is what makes one-click installation possible away from
  Debian-like systems.
### Fixed

- **A built package could be thrown away instead of published.** The job that
  attaches a package to the release waited ten minutes for another workflow to
  create that release, then gave up with "release never appeared — build.yml
  likely failed". The diagnosis was wrong: on a repository whose `.deb` takes
  longer than ten minutes to build, the release simply did not exist yet, and a
  package that had built perfectly was discarded. Four modules reached v0.1.6
  with packages missing for some systems because of it. The job now creates the
  release itself when it is missing, so it no longer depends on another workflow
  finishing first.
### Added

- **Security policy and CI quality gate.** A `SECURITY.md` documents how to
  report vulnerabilities, and a CI workflow enforces `clippy -D warnings`, a
  dependency-vulnerability audit (`cargo audit`) and the frontend typecheck/tests.

### Security

- **Tasks now authenticates proxied requests from a signed token instead of trusting plain headers.**
  Requests must carry a valid `X-Kubuno-Auth` token minted by the core with this module's internal
  secret (see `kubuno-modauth`), rather than reading `X-Kubuno-User-*` headers at face value.

## [0.1.6] - 2026-08-19

### Changed

- **Pill-shaped buttons are gone from the interface.** Filter chips, view
  segments, tab selectors and action buttons that were drawn as pills now use the
  same 4 px corner radius as every other button — the shape set them apart for no
  reason other than habit. Round buttons that hold a lone icon, avatars, status
  dots and non-clickable badges keep their shape: a circle around a single glyph
  is not a pill.

- Theme tokens: two colours for navigation labels (`--color-text-nav`,
  `--color-text-nav-active`). Every module carries the same token sheet, so the
  values must match across them — whichever bundle loads last would otherwise
  win. No visible change inside this module.

### Changed

- Default application background token aligned with the core (`--body-bg` `#f8fafd`). Only
  visible when the module runs standalone: inside the shell the active theme sets it.

[Unreleased]: https://github.com/kubuno/tasks/compare/v0.1.8...HEAD
[0.1.8]: https://github.com/kubuno/tasks/releases/tag/v0.1.8
[0.1.7]: https://github.com/kubuno/tasks/releases/tag/v0.1.7
[0.1.6]: https://github.com/kubuno/tasks/releases/tag/v0.1.6
