# Security Policy

## Supported versions

Only the latest release is supported with security fixes.

| Version | Supported |
|---|---|
| Latest release | ✅ |
| Older releases | ❌ |

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Report them
privately through GitHub's
[Security Advisories](https://github.com/rzlco666/rizal-agent-hq/security/advisories/new)
for this repository. You should receive a reply within a few days; a fix or a
mitigation plan will follow as fast as reasonably possible.

## Design notes

- HQ binds to **127.0.0.1 by default** — it is a personal dashboard. If you
  expose it beyond your machine (e.g. a Tailscale Serve or a reverse proxy),
  access control is that layer's responsibility.
- HQ only **reads** the data files of the tools it monitors (session logs,
  stats databases) and **redacts** secrets from free text before display.
- No telemetry: HQ never phones home. The only outbound calls are the ones
  *you* configure (for example an LLM summary provider or an alert bridge).
