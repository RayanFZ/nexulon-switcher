# Contributing

Thanks for helping improve Nexulon Switcher.

## Reporting a bug

Open an issue with the **Bug report** form and include:

- what you did, what happened, and what you expected;
- the version from the **About** panel and how you installed it (MSI or portable);
- your Windows version;
- the relevant lines from `%APPDATA%\Nexulon Switcher\logs\nexulon-switcher.log`.

Please **do not** attach saved login folders, `RiotGamesPrivateSettings.yaml`,
browser profiles, or tokens. Those are credentials. Redact account names if you
would rather not share them.

## Suggesting a feature

Use the **Feature request** form. For a new launcher or service, say where it
stores its login files and which process should be closed on a switch.

## Code

Build instructions and the source layout are in the
[README](README.md#build). Keep the app native and compact: no local web server,
no extra runtime, no telemetry. Run `cargo test` before sending a pull request.
