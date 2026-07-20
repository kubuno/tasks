<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Kubuno Tasks

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**Kubuno Tasks — tasks and Kanban boards (Tasks + Deck)**

A module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform.

## Features

- **Kanban boards & task lists** — drag-and-drop stacks, a flat list view, subtasks, comments, labels, assignees, priorities, due dates and recurrence; cards and rows are tinted with the task color for at-a-glance scanning.
- **Smart collections** — Today, Upcoming, Overdue, Important… each collection is addressable by URL (`/tasks/#collection/today`), so direct links and the browser Back button both work.
- **Quick task creation from anywhere** — a globally mounted "New task" dialog is published as a platform service (`tasks.createTask`), so other modules (chat, notes…) can create a task without leaving their own view. Consumers degrade gracefully when Tasks is not installed.
- **Cross-module task cards** — "Copy for Kubuno" puts a rich JSON envelope of the task on the clipboard; pasting it into another module (chat, notes…) renders an interactive task card that deep-links back to the task (`?task=<id>` opens the detail panel). Tasks can also be tagged with the platform-wide Kubuno labels.
- **Delta sync for local-first clients** — `GET /boards/delta` and `GET /tasks/delta` stream owner-scoped changes past a monotonic cursor (live rows + tombstones, paginated), and create endpoints honour client-minted UUIDs, so offline clients can replay their local changes and pull the server state incrementally.
- **CalDAV & interop** — CalDAV synchronization, per-task iCalendar (`.ics`) export, calendar overlay integration.
- **Per-user settings** — display density, default view, completed-task visibility and grouping, stored per user.

## Architecture

A standalone Rust process that registers with the [core](https://github.com/kubuno/core) at startup; the core proxies its routes (`/api/v1/tasks/*`) and serves its runtime-loaded React frontend bundle.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, schema `tasks`); migrations in `migrations/`.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@kubuno/ui` and `@kubuno/drive` from npm (provided by the host at runtime via the import map).

## Install

This module ships in the **all-in-one [Kubuno](https://github.com/kubuno/core) Docker image** (`ghcr.io/kubuno/kubuno`) — the easiest way to self-host a full Kubuno instance (core + every module). See **[kubuno/docker](https://github.com/kubuno/docker)** for `docker compose` instructions.

Native packages are also published on each [GitHub Release](https://github.com/kubuno/tasks/releases):

- **Debian/Ubuntu** — `kubuno-tasks_*.deb`
- **Fedora/RHEL/openSUSE** — `kubuno-tasks-*.rpm`
- **Windows** — `kubuno-tasks-setup-*-x64.exe` (NSIS installer; deploys into the existing core installation and restarts the service)
- **macOS** — `kubuno-tasks-*.pkg` (installs under `/usr/local/kubuno/modules/` and restarts the launchd daemon)

To build these packages from source, see below.

## Build

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16.

```bash
cargo build --release                     # → target/release/kubuno-tasks
cd frontend && npm ci && npm run build     # → dist/{entry.js, entry.css}
bash build_deb.sh                          # → dist/kubuno-tasks_*.deb
```

Platform-specific packages (same self-detecting layout as the `.deb`, so the core discovers the module identically):

```bash
bash build_rpm.sh       # → dist/kubuno-tasks-<ver>-1.<arch>.rpm   (needs rpmbuild)
bash build_windows.sh   # → dist/kubuno-tasks-setup-<ver>-x64.exe  (needs NSIS; native or cargo-xwin cross-build)
bash build_macos.sh     # → dist/kubuno-tasks-<ver>-arm64.pkg      (run on macOS; UNIVERSAL=1 for a fat binary)
```

CI builds all of them: `build.yml` produces the `.deb` and `dist.yml` the RPM/Windows/macOS artifacts, attached to the GitHub Release on every `v*` tag.

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui`, `@kubuno/drive` from the `@kubuno` npm scope.

## License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
