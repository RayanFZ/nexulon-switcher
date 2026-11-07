# Nexulon Switcher

A compact native Windows account switcher for game launchers, browsers and
desktop apps. It keeps the habit that matters — **double-click an account to
sign in** — and replaces the TcNo Account Switcher front end with a smaller,
faster app with its own look.

- One ~2.9 MB executable. The window is a native Win32 window hosting a
  Microsoft Edge **WebView2** surface; the page, styles and scripts are compiled
  into the exe, so there is no local web server and no loose web files.
- **Double-click an account** to sign in. There is no separate Login button.
- Platform launchers live in the top bar. By default they collapse into one
  **folder chip per category** — **Launchers**, **Games**, **Browsers**, **AI**,
  **Chat**, **Cloud**, **Creation**, **Media** and **Other** — so 45 services
  fit without a long sideways scroll. Click a folder to pick a service, or turn
  the folders off in Settings → Platforms to get the flat chip strip back.
- **45 services out of the box**: game launchers and games, ten browsers
  (Chrome, Edge, Firefox, Brave, Vivaldi, Chromium, LibreWolf, Waterfox, plus
  Chrome Beta and Canary), AI agents (Claude Code, Claude Desktop, ChatGPT),
  communication (Discord, Slack, Teams, Telegram, Signal), productivity
  (Notion), media (Spotify) and more.
- Browsers are switched by **opening a chosen profile** instead of copying
  gigabytes of profile data into the app's data folder.
- Every account can carry its own **avatar and card cover**. For Riot accounts
  the Valorant player card can be fetched once, opt-in, from the signed-in
  client.
- Riot accounts can also show a **rank, RR and level badge** on the card, with
  peak rank, title, player card and sprays in a profile drawer.
- **Steam** personas, **Chrome** Google names and pictures, **Firefox** account
  names and **Claude** emails are read from local files, so cards fill in
  themselves without any network call.
- No corporate branding, no build date, no version string in the chrome.
- Deleting an account is recoverable: sessions move to a trash folder and can be
  restored from the Undo toast.

## Requirements

| | |
| --- | --- |
| OS | Windows 10 or Windows 11 (64-bit) |
| Runtime | Microsoft Edge WebView2 (Evergreen). The MSI bundles the WebView2 bootstrapper and installs it silently when the machine lacks it. Windows 11 already ships it. |
| Portable exe | No installer, no admin |
| MSI | Per-machine install to `C:\Program Files`, one UAC prompt |

The exe links the static C runtime, so it does not need the Visual C++
redistributable.

## Install

### Portable

Run `Nexulon Switcher.exe` from anywhere. Settings and saved logins live in
`%APPDATA%\Nexulon Switcher`, so the exe itself stays portable. The WebView2
profile is written to `%LOCALAPPDATA%\Nexulon Switcher\WebView2`, never next to
the exe — that is what lets the same binary run from `Program Files`.

### MSI

Double-click `Nexulon Switcher Setup.msi` and approve the UAC prompt.

- Installs for all users to `C:\Program Files\Nexulon Switcher`.
- Adds a common Start Menu shortcut and a public desktop shortcut.
- Registers in Apps & features for a clean uninstall.
- Runs `redist\MicrosoftEdgeWebView2Setup.exe /silent /install` only when no
  machine-wide WebView2 runtime is found; the runtime stays installed after the
  switcher is removed because other apps rely on it.

## Using it

- The top bar starts as **category folders**. Click a folder to open its
  services and pick one; the folder shows the active service's icon. Switch to
  the flat strip in **Settings → Platforms → Group services into folders** if
  you prefer every chip on one row. Either way the rail scrolls sideways on a
  narrow window and the chips shrink to icons.
- **Double-click an account** (or select it and press `Enter`) to sign in. A
  **single click** on an account that has a Riot profile opens that profile;
  the short delay lets a double-click cancel it and sign in as before.
- **Grid / Tiles / Details** in the platform header switches how the roster is
  laid out. Tiles fit more accounts on screen and Details turns each card into
  a compact row; the choice is remembered across launches.
- The card face shows the name, local name and badges only. The **account ID**
  is kept out of the way in the `⋯` menu header, with **Copy account ID** below
  it. The same menu opens **Rename / Notes / Open folder / Delete**, plus the
  **Artwork** group: *Add or change picture*, *Add or change cover*, and the
  matching remove actions.
- **Add account** captures whatever is signed in right now; **Detect folder**
  finds the launcher installation.
- The gear opens settings: **General** (launch after switch, clear old login,
  start with Windows), **Look** (theme, accent, density, motion), **Platforms**
  (services grouped by category, with launcher folders), and **About**.

### Artwork

Each account card can show a small **avatar** next to the name and a wide
**cover** across the top. Both are chosen per account from the card menu and
stored in the app's data folder, so they survive renames and reinstalls. The
cover fades into the card, and the avatar overlaps it, so the name still reads.
Every card keeps a small avatar circle: if you have not chosen a picture, the
cover art is cropped into the circle instead of leaving an empty slot or
dropping initials onto a banner. Fetching a Valorant card also saves the card's
square icon as the avatar, which then takes over the circle.

### Profile-mode platforms

Chrome and Firefox are not copied account by account; they are switched by
launching the browser with a chosen profile:

| Platform | Profile source | Launch |
| --- | --- | --- |
| Chrome | `%LocalAppData%\Google\Chrome\User Data` | `chrome.exe --profile-directory=<folder>` |
| Chrome Beta / Canary | `...\Google\Chrome Beta` / `...\Google\Chrome SxS` | `chrome.exe --profile-directory=<folder>` |
| Edge | `%LocalAppData%\Microsoft\Edge\User Data` | `msedge.exe --profile-directory=<folder>` |
| Brave | `%LocalAppData%\BraveSoftware\Brave-Browser\User Data` | `brave.exe --profile-directory=<folder>` |
| Vivaldi | `%LocalAppData%\Vivaldi\User Data` | `vivaldi.exe --profile-directory=<folder>` |
| Chromium | `%LocalAppData%\Chromium\User Data` | `chrome.exe --profile-directory=<folder>` |
| Firefox | `%AppData%\Mozilla\Firefox\Profiles` | `firefox.exe -profile <path>` |
| LibreWolf / Waterfox | `%AppData%\librewolf\Profiles` / `%AppData%\Waterfox\Profiles` | `-profile <path>` |

These platforms show a **Profiles** chip instead of *Add account*. **Reload
profiles** re-reads the list, **Open this profile** launches it, and the last
opened profile is remembered as the current one. Rename and delete are disabled
because the profile belongs to the browser, not to the switcher.

### Service groups

Every platform definition can carry a `Category`. The top rail and **Settings →
Platforms** use it to group services into *Game launchers*, *Games*, *Browsers*,
*AI agents*, *Communication*, *Cloud & productivity*, *Creation*, *Media* and
*Other*. Packs written before categories existed are grouped by name, so an old
`Platforms.json` still lands in the right section.

Installed services show a green **Ready** chip; the rest show **Missing** and
can be pointed at the right folder from Settings → Platforms. Copy-mode apps
that keep a normal login folder (Telegram, Signal, Slack, Teams, Notion,
ChatGPT) use the same save/restore engine as game launchers.

By default the rail shows one folder per category so the whole catalogue is
reachable without a long scroll. **Group services into folders** turns that on
or off, and **Show only installed services** trims the folders and search
palette to installed services; the service that is currently open always stays
visible.

### Local names and pictures

Cards are filled in from the apps themselves, with no network and no tokens:

| Platform | Read from | Used for |
| --- | --- | --- |
| Steam | `config\loginusers.vdf`, `config\avatarcache` | persona name and cached avatar |
| Chrome | `Local State` → `profile.info_cache`, `Google Profile Picture.png` | Google name and picture |
| Firefox | `prefs.js` (`identity.fxaccounts…`) | Firefox Account display name |
| Claude Code | `~\.claude.json`, `~\.claude\.credentials.json` | email and plan |
| Claude Desktop | `%LocalAppData%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\config.json` | account marker |

These are fallbacks: artwork you choose yourself always wins. The saved folder
name stays the account name, so switching still targets the right session.

### Valorant player cards

On a Riot account, **Fetch Valorant card…** asks Riot which card the account
that is currently signed in to the Riot Client has equipped, downloads that
card's art from valorant-api.com, and stores it as the account's cover. The
card's square `displayIcon` is saved as the avatar at the same time, so the
initials are replaced by real art.

This is opt-in and deliberately narrow:

- Only the **currently signed-in** client is read. Saved accounts' tokens are
  never touched.
- When the Riot Client is open, the app uses the client's own loopback API
  (`/entitlements/v1/token` on the lockfile port) and never touches the saved
  tokens, so the client can stay running.
- With Riot closed, the app falls back to one refresh-token exchange. Riot
  rotates the refresh token when it is exchanged; the app writes the new tokens
  back and keeps a `RiotGamesPrivateSettings.yaml.nexulon-backup` copy, but an
  interrupted run may ask you to sign in to Riot again.

The dialog names the live account before anything is fetched, and warns when it
differs from the card you are applying the art to.

### Riot profile, rank and level

**Refresh Riot profile…** on a Riot account collects the current competitive
tier and RR, the peak tier recorded across seasons, the account level, the
equipped title, player card and sprays. It is a single token exchange, so one
refresh rotates the Riot sign-in once instead of once per field.

- The card then shows a **rank chip** with the tier icon and RR, plus a
  **Lv** chip. An account with no rank this act shows an **Unranked** chip.
  **View Riot profile** opens a drawer with peak rank, current rating, title,
  player card, sprays and a per-act season history.
- Only the account **currently signed in** to the Riot Client can be refreshed.
  Ranks for saved accounts are snapshots from the last time each one was live;
  there is no third-party tracker and no Riot ID is sent anywhere.
- Before writing, the app compares the live account's PUUID (or name) with the
  saved account. On a mismatch nothing is saved: a second dialog names the live
  account and offers **Refresh anyway**.
- Tier names, rank icons and act names come from valorant-api.com and are cached
  for a week in `Profiles\Riot Games\catalog.json`; rank icons are cached as
  `rank-<tier>.png`. Listing accounts never waits on the network.
- Peak rank is the highest tier Riot recorded for a completed act. Riot does not
  expose the highest point reached inside a season, so a true in-season peak is
  not available without match-by-match history.

### Keyboard

| Keys | Action |
| --- | --- |
| `Ctrl+K` | Command palette: every account and action |
| `Ctrl+F` or `/` | Focus the search box |
| `Ctrl+1` … `Ctrl+9` | Jump to platform 1–9 |
| `←` `→` `↑` `↓` | Move across the roster |
| `Enter` | Sign in to the selected account |
| `Esc` | Clear search, close a menu, or close settings |

### Look and feel

Five themes (Nebula, Aurora, Ember, Frost, Mono), six accents, comfortable or
compact cards, and a reduce-motion switch. Every account card takes a hue from
its own name, so the roster reads at a glance. Three roster layouts — **Grid**,
**Tiles** and **Details** — are switched from the platform header and
remembered across launches. The native Windows title bar is repainted through
DWM to the active theme's background, text and border colours, so there is no
second dark strip above the app and no repeated wordmark.

### Upgrade features over the TcNo front end

| Feature | What it does |
| --- | --- |
| Command palette | `Ctrl+K` fuzzy search across accounts, platforms and actions |
| Pinned favourites | Pin accounts so they sit above the rest of the roster |
| Sort modes | Recent first, Name A–Z, Name Z–A, Most used |
| Live search | Filters names, account IDs and notes as you type |
| Per-account notes | Free-text notes stored with the saved session |
| Per-account artwork | Avatar and wide cover image for every account, chosen from the card menu |
| Valorant player cards | Opt-in fetch of the equipped card art, plus its square icon as the avatar, for the signed-in Riot account — with the Riot Client left open |
| Riot rank badges | Current tier, RR and account level on the card, with an Unranked chip when there is no rank |
| Riot profile drawer | Single click opens peak rank, title, player card, sprays and per-act season history |
| Roster views | Grid, Tiles and Details layouts, remembered per install |
| Local metadata | Steam, Chrome, Firefox, Claude Code and Claude Desktop names and pictures read from local files |
| Browser profiles | Chrome and Firefox switch by launching a chosen profile, with no profile copies |
| Copy ID / Open folder | One click to the account id or its folder on disk |
| Recoverable delete | Delete moves the session to `.trash` with a typed-name confirmation; the toast offers **Undo**, and restore puts the account id back |
| Activity drawer | The switch log, in the window, without opening a file |
| Themes and density | Five themes, six accents, compact mode, reduce motion |
| Toasts | Non-blocking results with an action button where one makes sense |
| Collapsible rail | Launcher chips become icons instead of overlapping |
| Installed-only rail | Optional switch hides services that are not installed, while keeping the open one visible |
| Category folders | One folder chip per service group, with a popover of that category, so 45 services never need a long scroll |
| Grouped services | Rail and settings group 45 services into launchers, games, browsers, AI, chat, cloud, creation, media and other |
| Blended title bar | DWM paints the native caption in the active theme, so the title bar and app read as one surface |
| Quiet account IDs | IDs and tags are hidden from card faces and live in the account menu header |
| More browsers | Edge, Brave, Vivaldi, Chromium, LibreWolf, Waterfox, Chrome Beta and Canary switch by profile |

## Data

Everything the app owns lives in one folder:

| Path | Contents |
| --- | --- |
| `%APPDATA%\Nexulon Switcher\LoginCache` | saved logins |
| `%APPDATA%\Nexulon Switcher\LoginCache\<Platform>\.trash` | deleted sessions awaiting Undo |
| `%APPDATA%\Nexulon Switcher\Artwork\<Platform>` | per-account avatars and covers, with an `index.json` |
| `%APPDATA%\Nexulon Switcher\Profiles\<Platform>\index.json` | cached Riot rank, level, title, card and sprays per account |
| `%APPDATA%\Nexulon Switcher\Profiles\Riot Games\catalog.json` | cached tier and act names (7 days) |
| `%APPDATA%\Nexulon Switcher\Profiles\Riot Games\rank-<tier>.png` | cached rank icons |
| `%APPDATA%\Nexulon Switcher\Platforms.json` | platform definitions |
| `%APPDATA%\Nexulon Switcher\WindowSettings.json` | enabled platforms and window state |
| `%APPDATA%\Nexulon Switcher\Settings\*.json` | per-platform settings |
| `%APPDATA%\Nexulon Switcher\Settings\Nexulon Switcher.json` | app preferences and UI state |
| `%APPDATA%\Nexulon Switcher\logs\nexulon-switcher.log` | switch log |
| `%APPDATA%\Nexulon Switcher\.initialized` | first-run marker |
| `%LOCALAPPDATA%\Nexulon Switcher\WebView2` | WebView2 browser profile |

On first launch the app creates that folder and, if
`%APPDATA%\TcNo Account Switcher` exists, copies `Platforms.json`,
`WindowSettings.json`, `LoginCache` and `Settings` into it once. The marker file
guards the import: after that the TcNo folder is never read again, so the old
app can be uninstalled safely.

A trashed session keeps a small `.nexulon-trash.json` marker holding its name and
account id, because launchers differ in which file stores the id. That is what
lets **Undo** restore both the session and the id mapping.

Artwork is keyed by account **name**, not by folder, because a profile-mode
platform has no login folder of its own. Renaming an account moves its artwork
with it, and the files are named from a hash of the name.

The bundled platform pack is merged **additively**: on startup any platform in
`data\Platforms.default.json` that is missing from `Platforms.json` is added and
enabled. Existing entries are never overwritten, so local edits and folders
survive an upgrade.

`NEXULON_DATA_DIR` overrides the data root (used by the sandbox test) and skips
the one-time import.

## Build

```powershell
cargo build --release
```

The binary lands in `target\release\nexulon-switcher.exe`. `build.rs` embeds
`assets\nexulon.ico` plus version information. `.cargo\config.toml` enables the
static CRT. To keep your checkout path out of the binary, add a remap for your
own build:

```powershell
$env:RUSTFLAGS = '-C target-feature=+crt-static --remap-path-prefix=<your-checkout>=C:\build'
cargo build --release
```

### MSI

WiX 3.14 binaries are not committed. Download and extract
`wix314-binaries.zip` from <https://github.com/wixtoolset/wix3/releases>, then:

```powershell
powershell -ExecutionPolicy Bypass -File installer\build_msi.ps1 -WixDir D:\path\to\wix314
```

The script copies the release exe to `installer\payload`, compiles the WiX
source, and writes `Nexulon Switcher Setup.msi` next to the project. It refuses
to build when `installer\redist\MicrosoftEdgeWebView2Setup.exe` is missing.
Installer artwork can be regenerated with `python tools\make_installer_art.py`.

## Layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | tao window + wry WebView2 host, DWM caption colours, `WebContext` profile path, IPC workers, `--dump` |
| `src/bridge.rs` | JSON dispatch (`bootstrap`, `accounts`, `switch`, `artwork_set`, `valorant_card`, `riot_profile`, `restore`, ...) |
| `src/engine.rs` | switching engine (copy out/in, clear, ids, registry, launch, profile mode, artwork, trash/restore) |
| `src/platforms.rs` | `Platforms.json` model, switch mode, category grouping, profile discovery and launch arguments |
| `src/config.rs` | data-folder resolution, WebView2 profile path, one-time TcNo import, platform-pack merge |
| `src/http.rs` | small WinHTTP client used for Riot and card art requests (no TLS crate) |
| `src/riot.rs` | opt-in Valorant card fetch: token exchange, write-back, region, loadout, art |
| `src/profile.rs` | cached Riot rank/peak/level/title/sprays, catalog and rank-icon cache |
| `src/localmeta.rs` | Steam/Chrome/Firefox/Claude names and pictures from local files |
| `src/dialog.rs` | native `GetOpenFileNameW` image picker |
| `src/assets.rs` | embedded platform SVGs, returned raw and tinted by CSS |
| `web/index.html` | app shell and overlays |
| `web/style.css` | theme tokens, layout, card covers, drawers, responsive rules |
| `web/app.js` | UI state, IPC, palette, settings, artwork, Valorant dialog, toasts |
| `assets/platforms/` | bundled platform SVGs |
| `data/Platforms.default.json` | bundled platform pack (45 services, grouped by category) |
| `installer/` | WiX source, license, artwork, WebView2 redist, build script |
| `tools/sandbox_test.py` | end-to-end engine test against a throwaway data dir |
| `tools/make_installer_art.py` | generates `installer\banner.bmp` / `dialog.bmp` |
| `tools/shot.py` | PrintWindow screenshot helper; `--pid` targets one process |
| `tools/winutil.py` | resize/click/wheel helper for exercising the window |

## Debug helpers

```powershell
# Inventory of platforms/accounts as JSON (the release build has no console,
# but --dump still writes to the attached terminal)
target\release\nexulon-switcher.exe --dump
target\release\nexulon-switcher.exe --dump accounts "Riot Games"
target\release\nexulon-switcher.exe --dump switch '{"platform":"Riot Games","account":"Name"}'
target\release\nexulon-switcher.exe --dump valorant_status
target\release\nexulon-switcher.exe --dump riot_profile '{"platform":"Riot Games","account":"Name"}'
target\release\nexulon-switcher.exe --dump artwork_set '{"platform":"Steam","account":"Name","kind":"cover","path":"D:/art.png"}'

# Full add/switch/rename/notes/artwork/delete/restore round trip in a sandbox
$env:NEXULON_EXE = "target\release\nexulon-switcher.exe"
python tools\sandbox_test.py

# Screenshot one window; --pid avoids matching another app by title
python tools\shot.py --pid 1234 D:\shot.png
```

`NEXULON_EVAL` runs one JavaScript snippet ~2.6 s after startup; it exists so the
interface can be driven by automated screenshot checks.

Never point `--dump switch` at real data unless you intend to switch.
`--dump valorant_card` will also be refused while a Riot process is running.

## Behaviour notes

- Switching kills the launcher processes, optionally clears the live login
  files, copies the saved account over them, and relaunches the launcher.
- Profile-mode platforms (Chrome, Firefox) never copy data: switching closes the
  browser and relaunches it with the chosen profile, then records that profile
  as `LastAccount`.
- The two switches in settings are persisted app-wide, not per platform.
- Rename/notes/delete operate on the saved logins in the Nexulon data folder.
- Delete never erases: the session folder is renamed into
  `LoginCache\<Platform>\.trash\<name>.<unix>`. Only an explicit trash purge
  would remove it, and the app does not offer one.
- Launcher folders can be typed in or found with **Detect**; paths support the
  usual `%AppData%`, `%LocalAppData%`, `%ProgramFiles%` variables.
- A `%Platform_Folder%` path falls back to the folder of the configured exe, so
  platforms with a non-standard install still find their login files.
- The Valorant fetch prefers the running Riot Client's loopback API, so the
  client can stay open and no token is rewritten. With Riot closed it uses the
  live refresh token, writes the rotated token back, and keeps
  `RiotGamesPrivateSettings.yaml.nexulon-backup` next to it. It never reads
  tokens for saved accounts.
- Riot profile refreshes follow the same rule and refuse to overwrite a saved
  account when a different account is signed in; the UI asks before forcing it.
  Rank data is a snapshot per account, updated only when that account is live.

## License

MIT — see [LICENSE](LICENSE). Use it, fork it, and ship your own build.
