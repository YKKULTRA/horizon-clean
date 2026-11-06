# Horizon Clean

An unofficial fork of [Horizon](https://github.com/horizon-rust/horizon) (Minecraft: Java Edition 26.3 rewritten in Rust) with all telemetry removed. Not affiliated with the Horizon project, Mojang or Microsoft. You need your own copy of Java Edition, exactly as with upstream.

There are no prebuilt downloads: everyone compiles it on their own computer (below).

Base: upstream commit `4aa6854a886183440256f4f4a4a9d1ed853227a1` (the `upstream` branch). `main` is that base plus this fork's changes.

## What is removed

In the game:

- The "I'm playing" ping: upstream's game sends `POST /v1/live` to `telemetry.playhorizon.dev` once a minute, without asking. The heartbeat thread is gone.
- Performance summaries and crash reports queued for upload. The outbox never writes, whatever an old consent file says. Ordinary local crash files and benchmark reports you ask for stay on your computer.
- The official server that upstream adds to your multiplayer list by itself. Servers you added are not touched.

In the shared telemetry crate (`crates/horizon-telemetry`): the HTTP dependency is gone, and the upload and live transports refuse to send. No setting, CLI command or `HORIZON_TELEMETRY_URL` value turns them back on.

In the launcher:

- The data-collection question, its settings and the report upload module.
- "N playing now" and the official server's player count.
- The sponsor banner and card (Sculk), the sidebar promotion and the sponsor/patch-notes prompts.
- Polling the release list at startup and every so often, and fetching `links.json` from the internet.
- Self-update and automatic game updates, so an upstream build never silently replaces this one.
- Hardware probing and memory sampling in ordinary sessions (a benchmark you start still measures, locally).
- Every game it launches gets `HORIZON_TELEMETRY_URL=off` and `HORIZON_NO_HEARTBEAT=1`, after per-instance overrides, so even an unmodified upstream build stays silent.

## Exploit Preventer (on by default)

A switch on the Horizon Settings page (`exploitPreventer` in `horizon-extras.txt`). With it on, a server learns what an unmodified vanilla client would tell it, and nothing else:

- **Sign translation exploit** (MC-265322, the fix NikOverflow's [ExploitPreventer](https://github.com/NikOverflow/ExploitPreventer) makes for the Java game). A server can put translation keys and key binds on a sign or an item's name, then read back what the client resolved through the sign editor or the anvil's rename box. Those lines now resolve only from vanilla's own strings and your resource packs': Horizon's and mods' strings come back as their keys, as from a vanilla client. A `keybind` component names vanilla's default key for a vanilla key mapping (never your own binding), and anything else comes back as its name.
- **Brand:** the client says `vanilla` in `minecraft:brand` instead of `horizon`, and so does a LAN world it hosts.
- **Local requests through resource packs:** a remote server's resource pack (and every redirect, and any DNS name) may not connect to this computer or its local network: loopback, private and link-local ranges, IPv6 unique-local, 100.64.0.0/10 (Tailscale and carrier NAT). The check is in the resolver, so nothing connects before it. A server on your own computer or network may still use local URLs.
- **Fingerprinting through the pack cache:** each account downloads server packs into its own `downloads/<account UUID>/`, so a server can't recognise you across accounts by what is already cached.

Also changed: Discord Rich Presence ("Playing Horizon" on your profile) is off by default; turn it on in Horizon Settings.

With the switch on, Horizon-only features that look at the brand don't see a Horizon client: a Horizon LAN host treats you as vanilla (no native-mod messages, no quiet commands from it), and a host with "Horizon clients only" turns you away. Turn the switch off for those.

Not covered yet, so a server can still tell Horizon from vanilla in these ways:

- Configuration's known packs: Horizon answers that it knows none and asks for every registry entry in full; vanilla says it knows `minecraft:core`.
- Behaviour that differs from vanilla anywhere else (timing, packet order, physics edge cases). Horizon aims at vanilla parity, but parity is not proven against an anticheat.
- Plugin channels a mod registers on purpose (as with the Java mod, these are left alone).
- Saved worlds record `horizon` among their server brands in `level.dat`.

## What still uses the network

Only what you use: Microsoft/Xbox sign-in, Mojang assets and Java downloads, multiplayer, skins, mod registries and their CDNs, release browsing when you open it, log sharing to mclo.gs when you click it, and optional Tailscale/friends networking. Java mods and other programs you run have their own network behaviour; this fork is not a firewall.

## Not changed yet

The launcher's manual release downloads, its native-mod builds and `horizon-server update` still use upstream's repository (`horizon-rust/horizon`). They install or compile upstream's code, not this fork's. The launcher still starts those builds with telemetry turned off (above). Pointing them at this repository is planned.

## Build (Linux x86-64)

You need Rust stable (1.85 or newer), Python 3.10+, JDK 25 (for preparing the game's files from your Minecraft), a C compiler, and the ALSA and udev headers (`libasound2-dev libudev-dev` on Debian/Ubuntu, `alsa-lib systemd-libs` on Arch).

```sh
git clone https://github.com/YKKULTRA/horizon-clean
cd horizon-clean
bash CLEAN_BUILD.sh      # privacy tests, launcher tests, then the launcher and game builds
bash PACKAGE_CLEAN.sh    # a portable folder in ../dist/horizon-clean
../dist/horizon-clean/run-clean
```

`CLEAN_BUILD.sh` writes its log to `../build-state/`. Set `CARGO_BUILD_JOBS` to change how many jobs Cargo uses (default 8). Set `HORIZON_BUILD_WAIT_CMD` to a command that prints something while the build should wait (for example while a game is running); the build checks it every 30 seconds.

`run-clean` keeps accounts, settings, saves and prepared files in its own `userdata/` folder. It imports nothing from other launchers and installs no menu entry or service.

Sign-in uses Horizon's own public Microsoft app ID (configuration, not a secret). Set `HORIZON_MSA_CLIENT_ID` to use an Azure app of your own. Microsoft sign-in has not been tested on this fork yet.

## Verification

On Linux x86-64 with this source:

- `cargo test --locked -p horizon-telemetry`: 36 passed, 1 ignored (upstream's).
- `cargo test --locked --release --lib --bins --tests` in `launcher/`: 430 passed, 3 ignored (upstream's).
- Clippy with warnings denied on `horizon-telemetry` and the launcher.
- The game, `horizon-ctl`, `horizon-prepare` and `horizon-server` build, and each starts.
- Privacy regression tests (`crates/horizon-telemetry/tests/consent_gating.rs`, `launcher/tests/clean_privacy.rs`) bind a real local listener and call every former telemetry transport with an explicit URL and all-enabled legacy consent: no connection is made.

This is not a full security audit or a gameplay test. Sign-in, the first preparation of Minecraft's files, rendering and joining a server have not been exercised yet.

## Upstream

Syncing with upstream happens by hand: `main` is rebased onto a newer upstream commit, then the privacy tests run again before it is pushed.

## License

MIT, like upstream. See [LICENSE](LICENSE); upstream's copyright notice stays.
