# Pending implementation

Work still left on Engineering Copilot. The floating teddy, night mode, AI provider setup, and the Jira task-to-branch flow are already in the app.

## Start here

Do these before adding more live integrations.

- [ ] Remember where the teddy sits on screen, and restore that position on the next launch.
- [ ] Keep AI chat history. Today each message is a single prompt with no memory of earlier turns.
- [ ] Send the active ticket, approved plan, and a short git status with each AI request.
- [ ] Remove or hide the sample Brief, Tickets, and PRs tabs until they show live data.
- [ ] Replace `csp: null` with a real content security policy before more outside content is shown.

## Connect live tools

- [ ] Read Slack mentions into the briefing. Store the token in Windows Credential Manager. Read only.
- [x] Show the signed-in user's open GitHub pull requests instead of the sample list.
- [ ] Let the user pick a local git repository instead of typing an absolute path.
- [ ] Load more than the first 25 Jira issues, with assignee and status filters.
- [ ] Add optional launch when Windows starts. Off by default.
- [ ] Notify when Jira, Slack, or GitHub changes while the app is open.

## Copilot

- [ ] Open the current chat in Claude, ChatGPT (OpenAI), or Gemini. The user picks one, and the widget opens that app or site with the conversation so they can continue there.
- [x] Listen to a meeting and write a summary. Hover the teddy for Start listening or Stop listening. The teddy wears headphones while it listens. Stop opens the Meet tab, and Meet can stop listening too. It captures the microphone and the call playing on this PC, then saves decisions, action items, and a recap in the widget. It does not listen in the background, and it does not post the summary to Slack or Jira.
- [ ] Build a once-a-day morning digest of new tickets, reviews, and mentions.
- [ ] Summarize a pull request from its diff. Read only. Do not post the review.
- [ ] Draft a commit message and pull request body. Commit or push only after the user approves.
- [ ] Draft a Jira comment or status change. Send it only after the user approves.

## Memory and release

- [ ] Store chats, tickets, and plans in local SQLite so more than one ticket and past chats survive restarts.
- [ ] Save source-linked memories, each pointing back to the Jira issue, message, or file it came from.
- [ ] Expose those memories and the active plan through an MCP server for Cursor and Claude.
- [ ] Code-sign the Windows installer so SmartScreen does not treat it as an unknown app.
