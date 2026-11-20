# Security policy

Nexulon Switcher stores launcher sessions, so security reports are taken
seriously.

## Supported versions

Only the builds published on the
[releases page](https://github.com/RayanFZ/nexulon-switcher/releases) are
supported. Each fix replaces the current release assets, so always test the
latest download before reporting.

## Reporting a vulnerability

Do **not** open a public issue for a security problem.

Use the repository's **Security → Report a vulnerability** button
(<https://github.com/RayanFZ/nexulon-switcher/security/advisories/new>), which
opens a private advisory visible only to the maintainer.

Please include:

- the app version from the About panel and how it was installed (MSI or portable),
- the Windows version,
- what an attacker could do, and the steps to reproduce it,
- the relevant lines from `%APPDATA%\Nexulon Switcher\logs\nexulon-switcher.log`.

Never attach saved login folders, `RiotGamesPrivateSettings.yaml`, browser
profiles, tokens or anything else from `LoginCache`. Redact account names if you
would rather not share them. A proof of concept can be described in words
instead.

## Scope

In scope: the Rust host, the IPC bridge, the bundled web UI, the MSI and the
release binaries.

Out of scope: Riot Games, Microsoft WebView2 and third-party launchers
themselves; problems that require an already-compromised Windows account; and
reports that only restate the licence (modification and redistribution are not
permitted, which is intentional).

## What to expect

Reports are acknowledged as soon as possible, usually within a few days. If a
fix is needed it ships as a replacement of the current release, with the new
hashes added to the release notes.
