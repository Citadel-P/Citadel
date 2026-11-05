---
name: citadel-feature-workflow
description: Implement or review Citadel features using the repository's established Rust, persistence, server, and React integration patterns. Use when adding a feature or changing its module organization, API wiring, resource pages, forms, or realtime synchronization.
---

# Citadel feature workflow

Work from the repository root. Follow `AGENTS.md` and the relevant sections of
`docs/ARCHITECTURE.md`; this skill supplies the comparison workflow, not a second
architecture specification. User instructions determine scope and take precedence.

## Choose an implemented reference

Before editing, inspect the closest working feature through the affected layers.
Deployments and Swarm services are useful references for durable resources;
choose a smaller feature when its responsibilities are a closer match. Name the
reference and the patterns being reused in the implementation plan. Read actual
implementations, not just file trees.

A new contributor should find equivalent responsibilities under familiar names.
Follow the reference's module organization, public exports, error handling,
dependency wiring and test placement. Add files for actual responsibilities;
do not copy an entire scaffold or create empty modules. Follow the documented
`name/mod.rs` convention when a Rust module has child files.

Keep new or refactored `lib.rs` and `mod.rs` files thin: module declarations,
deliberate re-exports, crate/module documentation and applicable attributes.
Put implementation in named sibling files
organized by responsibility. Plan that split before adding orchestration, tool I/O
or lifecycle/cleanup logic; do not accumulate them in the entry point and defer
the split until review. Existing large crate roots or module entry points are not
templates for new work.
Choose names from the actual responsibilities, not a mandatory file list; keep
related helpers together and avoid one file per function or empty scaffolding.

## Rust, infrastructure and server

Trace only the relevant path: HTTP input/view → feature use case and ports →
persistence/runtime adapter → composition → worker and realtime publication.

- Use the owning feature's models and typed contracts. Keep a release checker,
  operation executor and repository distinguishable by their actual roles.
- Compare PostgreSQL queries, authorization, claims and Activity writes with the
  reference. Splitting modules must preserve complete atomic transactions.
- Keep execution-specific evidence with its mechanism; do not claim a model is
  backend-independent while it requires a particular container or filesystem.
  Preserve existing persisted formats unless the task includes changing them.
- Use the established typed notifier and task-ownership patterns where applicable.
  Put construction in composition, scheduling in workers, and presentation in
  API resources. Shared views consume data, not live service dependencies.

## Frontend

Inspect `src/frontend/src/features/index.tsx`, the relevant `src/frontend/src/pages/`
components, and an equivalent feature's screen, form, data hook and realtime path.
For workloads, start with deployments or Swarm services; for tabbed pages, also
inspect access or bindings.

- Register components through the existing resource page/form/detail factories.
  Let the page abstraction own routing, tabs and layout where it supports the
  feature. Prefer declarative page options over a feature-specific page wrapper.
- Use the shared FormShell/FormBuilder and existing validation/save behavior.
- Use generated API clients and established query keys. Identify whether each read
  is synchronized by a realtime snapshot or an invalidation event, including join
  and reconnect recovery, before adding mutation callbacks.
- Reuse realtime synchronization. Do not add polling, refresh-all helpers or
  mutation-triggered HTTP reloads for data already covered by it. Applying an
  authoritative mutation response to cache is valid; preserve stale-read
  cancellation and disconnected-state behavior where needed. Error retry actions
  are separate from routine synchronization.
- Reuse shared resource loading/error states, actions and appearance tokens.
  Keep API/UI primitives independent of feature screens and app routing.

## Verify the integration

Compare the finished path with the chosen reference: entry points, naming,
ownership, authorization, persistence-before-notification and frontend cache
updates. Explain necessary deviations in the final response; do not seek approval
for routine implementation choices already authorized by the task.

Inspect each new or expanded `lib.rs` and `mod.rs` before finishing. Move implementation into
the appropriate responsibility module while preserving public import paths,
minimal visibility, cancellation ownership and cleanup ordering. Keep unrelated
existing modules outside the task scope.

Run the applicable checks from `docs/DEVELOPMENT.md` and `test/README.md`, including
architecture tests when boundaries change. Prefer behavioral tests for access,
commit/notification ordering, realtime/reconnect and stale reads. Extend architecture
guards for enforceable dependency rules, not an exact list of required filenames.
