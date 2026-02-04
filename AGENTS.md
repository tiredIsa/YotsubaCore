# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Vue 3 UI. Entry is `src/main.ts`, root component is `src/App.vue`, and state lives in `src/stores/` (Pinia).
- `src/assets/` and `public/` contain static assets used by the frontend.
- `src-tauri/src/` contains the Rust backend and Tauri commands.
- `src-tauri/resources/` is for bundled binaries like `sing-box.exe` (kept out of git).
- `src-tauri/capabilities/` and `src-tauri/tauri.conf.json` define permissions and app config.
- `dist/` is the Vite build output (generated).

## Build, Test, and Development Commands
- `bun run dev`: start the Vite dev server for the UI.
- `bun run build`: typecheck (`vue-tsc --noEmit`) and build the web assets.
- `bun run preview`: serve the built frontend from `dist/`.
- `bun run tauri dev`: run the desktop app with hot reload.
- `bun run tauri build`: bundle the Windows app.
- `cargo check` (in `src-tauri/`): quick Rust compile check.

## Coding Style & Naming Conventions
- Use 2-space indentation in Vue/TypeScript and 4-space indentation in Rust.
- Follow existing conventions: double quotes and semicolons in TS/JS, `rustfmt` defaults in Rust.
- Components use `PascalCase.vue` (e.g., `App.vue`). Non-component modules are lowercase (e.g., `src/stores/proxy.ts`). Rust modules and fields use `snake_case`.
- No formatter config is enforced; keep changes consistent with adjacent code.

## Testing Guidelines
- Automated tests are not configured yet.
- If adding tests, prefer `*.spec.ts` for frontend (Vitest) and `*_test.rs` for Rust (`cargo test`).
- Document any new test command in this file and the README.

## Commit & Pull Request Guidelines
- No Git history is available in this directory. Use Conventional Commits where possible (e.g., `feat:`, `fix:`, `chore:`).
- PRs should include a clear description, linked issues, and screenshots or recordings for UI changes.
- Do not commit secrets, user profiles, or third-party binaries. Place `sing-box.exe` in `src-tauri/resources/` for local builds.

## Security & Configuration Tips
- Share links and proxy profiles can contain credentials. Keep them out of the repo and redact them in logs or screenshots.
- Only enable the minimum Tauri capabilities required for new features.
