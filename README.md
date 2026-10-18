# Engineering Copilot — Windows floating widget (Phase 1)

A **runnable Tauri 2 + React desktop project** with a floating draggable bubble, click-to-expand dashboard, system-tray menu, and a demonstration notification. **All ticket, Slack, and PR content is sample data.** Phase 1 does NOT connect to Jira, Slack, GitHub, any AI API or persistent memory.

## Prerequisites on Windows 10/11

- Node.js LTS, Rust (MSVC toolchain), Microsoft C++ Build Tools with Desktop development with C++, and WebView2 Runtime if not already installed.
- See https://v2.tauri.app/start/prerequisites/

## Run

Open this folder in VS Code, launch PowerShell in the folder, and run:

```powershell
npm install
npm run desktop
```

- **Click** bubble to expand. **Right-click and hold/drag** the bubble to move it; use the **⠿ handle** in the panel header to drag the expanded widget.
- Click **−** to collapse to the bubble.
- Use tray icon menu to show or quit.
- **Test notification** exercises the operating system notification integration (Windows may apply notification preferences).
- The text box records notes **in memory only for this running session**; it is not an LLM chat.

## Build Windows EXE and installer

```powershell
npm run package:win
```

Look for an executable at `src-tauri\target\release\engineering-copilot-widget.exe` and an NSIS installation executable in `src-tauri\target\release\bundle\nsis\`.

Or push the repository to GitHub and run the included `Build Windows EXE` GitHub Actions workflow, then download the workflow artifact. This requires a GitHub repository under your control and Actions enabled.

## Architecture roadmap

- Phase 2: OAuth / approved credentials and least-privilege Jira, Slack, GitHub integrations.
- Phase 3: change detection, event processing, daily digests and read-only PR analysis.
- Phase 4: local SQLite + vector retrieval, source-linked memories and an MCP server for Claude and Cursor.
- Phase 5: optional drafts and approval-gated external writes.

## Scope and safety

- Windows startup auto-launch is *not* implemented in this Phase 1 build.
- No background monitoring yet; the UI is local demo content.
- `csp:null` is only a prototype convenience; set a restrictive CSP before adding external content or production integrations.
- Windows NSIS installers from local builds are not code-signed; SmartScreen may flag untrusted binaries.
- Keep API tokens outside the frontend and use OS-protected credential storage in future phases.


## AI Provider onboarding (Phase 1.1)

Expand the bubble → **AI Setup**. Choose **OpenAI**, **Anthropic Claude**, or **Custom OpenAI-compatible API**. Select a suggested model or type another exact model ID; enter the API key and choose **Save configuration**. After saving, use the composer to send a prompt to the chosen provider.

- OpenAI: `POST https://api.openai.com/v1/chat/completions`.
- Claude: `POST https://api.anthropic.com/v1/messages`.
- Custom: expects a **full** chat completions endpoint URL, e.g. `https://example.com/v1/chat/completions`; JSON response must contain `choices[0].message.content`. For an unauthenticated local endpoint, enter `NO_KEY`.
- Keys are stored via the Rust `keyring` crate, backed by Windows Credential Manager. Nonsecret provider, model, and URL are written to the app config directory. They are never placed in `localStorage` or the config JSON.
- Settings work for one active provider at a time; credentials are stored separately per provider. To re-use a saved provider key, choose that provider and save the setting without re-entering it.
- AI chat currently sends **one prompt per request** (no conversation history or repository context); other tabs remain illustrative demo data.
- Suggested model IDs are examples; model availability depends on your API account and may change. Use **Other model ID** if needed.
- Custom endpoints can receive secrets, so use only trusted HTTPS servers (or HTTP localhost for local model servers). API calls may be billable.
- Requires connectivity to chosen hosted API and working Windows Credential Manager. Windows build has not been compiled or verified in this environment.

## Phase 1.2 — Jira task-to-repository workflow

Open the floating widget → **Task Flow**. Configure Jira Cloud site URL (HTTPS), Atlassian account email, project key and a Jira API token (kept in Windows Credential Manager). **Load tickets** fetches up to 25 unfinished issues, or enter the ticket key directly. **Pull ticket** fetches the latest summary, rich-text description and up to 20 returned comments. Discuss acceptance criteria with your chosen saved AI provider under **AI Setup**; generate a proposed implementation plan. Review it and explicitly click **Approve plan**. Choose and inspect a **local, existing Git repository root** (use an absolute Windows path, e.g. `C:\Users\you\dev\service`). After confirming it is the correct repository, click **Start work on branch**.

The app then creates an `ai/<ticket-key>` local branch and writes `.engineering-copilot/<ticket-key>-plan.md`, ready for implementation in Cursor/VS Code. This is a controlled **workspace preparation** step; it intentionally does **not** autonomously modify source code, run tests, commit, push, open PRs, or update Jira. The repository must have a clean working tree; an existing task branch is never overwritten. The process persists the single active workflow between launches in the user's application configuration folder. Git must be installed and available on `PATH`. Jira project search uses Jira Cloud REST API `/rest/api/3/search/jql`, and issue details use `/rest/api/3/issue/{key}`. Account/app permissions are required.

**Security:** Jira description and comments are sent to the selected LLM provider for planning. Use only with organizational approval. API tokens are stored using the OS keychain, never in the workflow JSON. Review the plan for inaccuracies or prompt injection before approving. Do not paste secrets into Jira discussions. Custom model endpoints can receive company data; configure only trusted endpoints.

**Known limits:** No repository discovery across GitHub organizations yet; repository selection is by local absolute path. Jira issue search returns a limited selection; enter a key to fetch a specific issue. No autonomous source-code modification or PR publishing in this phase.
