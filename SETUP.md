# Start and setup

Engineering Copilot is a Tauri desktop app: a Rust window around a React UI. From this folder, install the tools once, then start the floating widget with `npm run desktop`.

## 1. Install the prerequisites (once)

- Node.js LTS
- Rust, via [rustup](https://rustup.rs), with the MSVC toolchain
- Microsoft C++ Build Tools, with the **Desktop development with C++** workload
- WebView2 Runtime, which is already present on current Windows 10/11

Official list: https://v2.tauri.app/start/prerequisites/

On this machine Node is `v22.21.1`, Rust is `1.99.1`, and the C++ build tools are already installed. If a new PowerShell says `cargo` is not recognized, close it and open a fresh one so `%USERPROFILE%\.cargo\bin` is on `PATH`.

## 2. Install dependencies (once)

Open PowerShell in this folder and run:

```powershell
cd C:\Users\prasa\OneDrive\Desktop\engineering-copilot-phase1-2-jira-task-flow\desktop-engineering-copilot
npm install
```

## 3. Start the app

```powershell
npm run desktop
```

That starts the Vite dev server at `http://127.0.0.1:1420`, then compiles and opens the floating widget. The first Rust compile takes a minute or two. Later starts are faster.

## Using the widget

- Click the bubble to expand it.
- Right-click and drag the bubble to move it. When it is expanded, drag the ⠿ handle in the header.
- Click **−** to collapse it back to the bubble.
- Use the system-tray icon to show or quit the app.

## Build a Windows installer

```powershell
npm run package:win
```

The executable is at `src-tauri\target\release\engineering-copilot-widget.exe`. The NSIS installer is in `src-tauri\target\release\bundle\nsis\`.
