# Flowline

Hold-to-talk dictation for macOS and Linux, written in Rust with eframe/egui. Real
speech-to-text on your computer or on a server you choose — there is no demo mode,
no sample transcript and no seeded data.

## Installing

A release build made with `scripts/package.sh` (see below) is universal, so it
runs on Apple Silicon and Intel Macs with macOS 11 or later, with no account and
no other software.

1. Open `Flowline-<version>.dmg` and drag **Flowline** into **Applications**.
2. **First launch.** If the build is not notarized by Apple (a build made with
   the default scripts is not), macOS says it cannot verify the app. Right-click
   Flowline, choose **Open**, then **Open** again. Or run once:
   `xattr -dr com.apple.quarantine /Applications/Flowline.app`
3. **Speech model.** The on-device engine is inside a release build. Open **Settings →
   General → Transcription engine** and press **Download Small** (about 466 MB,
   from huggingface.co, once). Nothing else is ever downloaded and audio never
   leaves your Mac.
4. **Permissions** (macOS asks for each): Microphone to record, and
   Accessibility so the Fn key works anywhere and text can be pasted for you.
   Input Monitoring is optional.

Then hold **Fn** in any text field, speak, and let go.

## Installing on Linux

Flowline for Linux is a tarball. It needs a desktop with PipeWire or PulseAudio,
`libasound2` (present on any desktop with sound), and glibc 2.35 or newer
(Ubuntu 22.04, Debian 12, Fedora 36 or newer).

```sh
tar xzf flowline-<version>-linux-<arch>.tar.gz && cd flowline-<version>-linux-<arch>
sh install.sh            # optional: menu entry + icon, no root needed
sh flowline-setup.sh     # once: keyboard and paste access, then log out and in
./flowline               # or start it from your applications menu
```

Then open **Settings → Transcription engine** and press **Download Small**; the
speech engine itself is inside the archive.

**Shortcuts on Linux** (Linux apps cannot take keys away from each other, so
these are chords nobody else claims):

| | |
| --- | --- |
| Hold **Right Ctrl** (or Right Alt, or Ctrl+Space; pick in Settings) | dictate anywhere |
| **Ctrl+Alt+Shift+1…9** | run a Transform on the selected text |
| Hold **Ctrl+Alt+Space** | command mode |
| **Esc** / **Ctrl+Q** | cancel / quit |

How it works there, and what is different from macOS:

- The shortcuts read your keyboard through the kernel (evdev), so they work on
  X11 and Wayland. That needs the one-time `flowline-setup.sh`, which adds you to
  the `input` group; it also lets Flowline press Ctrl+V for you through
  `/dev/uinput`. Without it the app still works inside its window, and dictated
  text is left on the clipboard.
- Selected text is copied with Ctrl+Insert, never Ctrl+C, so a terminal's running
  program is not interrupted. Pastes use Ctrl+V, Ctrl+Shift+V in a terminal, and
  Shift+Insert when the app in front is unknown.
- There is no tray item: closing the window keeps Flowline running, and starting
  it again brings the window back. On Wayland a window cannot be hidden, so it is
  minimized instead.
- The app in front is only known on X11, so per-app tone and the terminal-aware
  paste work there; on Wayland every app counts as "Other".
- My Voice's learn-from-corrections is macOS-only for now (it needs the text of
  the field you pasted into); its dictionary and statistics work everywhere.
- Meeting detection reads `pactl` (is another app recording?) and the process
  list; the prompt is a zenity or kdialog dialog if either is installed.
- Optional helpers: `zenity` or `kdialog` (prompts, file picker), `pactl` (mute
  other audio, meeting detection), `xdotool` or `wtype` (paste, if you cannot use
  the uinput setup).
- Speech runs on the CPU (no GPU). Models are stored in
  `~/.local/share/flowline/models`.

Linux status: new. It was built and tested in a Linux container (Ubuntu,
including real transcription, the keyboard and paste path against the kernel's
uinput, the clipboard under X11, and all pages rendering); it has not yet been
used on a physical desktop, so GNOME-on-Wayland behaviour in particular is the
least proven part.

## Building from source

```sh
git clone https://github.com/manu-tech-code/flowline.git && cd flowline

scripts/build-whisper.sh        # one-off: builds the speech engine (needs cmake, git)
scripts/bundle.sh --install     # builds dist/Flowline.app and copies it to /Applications
scripts/package.sh              # universal build + dist/Flowline-<version>.dmg
```

You need Rust (`rustup`), the Xcode command line tools, and, for a universal
build, both targets: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`.
`cargo run` works for development; a development run uses Homebrew's
`whisper-cpp` if the bundled engine has not been built.

On Linux, `scripts/linux-docker.sh package` builds the tarball inside Docker
(`scripts/linux-docker.sh image` first, then `whisper` once for the bundled
speech engine, then `package`); `scripts/linux-docker.sh test` runs the tests.
On a Linux machine, run `scripts/build-whisper-linux.sh` and
`scripts/linux-package.sh` directly.

On macOS, to distribute without the Gatekeeper warning, sign and notarize with your own
Apple Developer ID:

```sh
CODESIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
NOTARY_PROFILE=your-notary-profile scripts/package.sh
```

Release builds rewrite the builder's home folder out of the binary, so a build
you share does not carry your user name.

## Getting a model without the app

`scripts/install-whisper.sh` still downloads a model (and Homebrew's engine) from
the command line, for people who prefer that.

## Dictation

- Hold **Fn** (or the chosen shortcut) *anywhere* — the hub does not need focus.
- Release to transcribe. **Esc** cancels. **Ctrl Space** is the alternative.
- The floating pill is a separate, non-activating panel: it appears over other
  apps, even when the hub is hidden, and never steals focus.
- The transcript is cleaned up, dictionary replacements and snippets are
  applied, it is pasted into the app that was frontmost when you started, and a
  history entry is added. It is also appended to the Scratchpad.
- Without Accessibility (or when the frontmost app changed), the transcript is
  kept in the Scratchpad instead of being pasted.
- Inside the hub, Space (or **Start dictation**) works too, and double-tapping
  starts hands-free listening.
- When the engine fails, the error is shown and the clip stays available under
  **Retry audio**. Nothing is invented.

## Permissions

macOS grants these separately; Flowline shows each one live in **Settings →
System → Permissions**:

| Permission | Needed for | If missing |
| --- | --- | --- |
| Microphone | Recording | Asked the first time you record |
| Accessibility | Posting ⌘V into the frontmost app, and consuming Fn so it does not trigger the Globe action | Transcript kept in the Scratchpad |
| Input Monitoring | Listen-only keyboard tap when Accessibility is not granted | Shortcut does not start dictation |

**Fn vs. the Globe key:** with Accessibility granted, Flowline installs an
*active* keyboard tap and swallows the Fn flags change, so Fn no longer opens
emoji picker / Spotlight. Without Accessibility it falls back to a listen-only
tap and the system Globe action still fires; in that case use **Ctrl Space** or
set System Settings → Keyboard → *Press 🌤 key to* **Do Nothing**.

## Transcription engines

**On this Mac** (default) runs `whisper-cli` on the recorded 16 kHz mono WAV and
returns its text. Fully offline. The model list comes from
`~/Library/Application Support/Flowline/models/`.

**Server** uses any OpenAI-compatible `/v1/audio/transcriptions` endpoint. Only
the endpoint typed in Settings is ever contacted: there is no hidden fallback to
another machine, and the list clears while a new endpoint is checked. A remote
host (Tailscale, home lab) is allowed and labelled *“audio is sent to …”*.
`FLOWLINE_API_KEY` adds a bearer token; `FLOWLINE_ENGINE_URL` / `FLOWLINE_MODEL`
override the saved settings for one run.

**Ollama and Kokoro are not transcribers.** Ollama serves chat/vision/embedding
models and answers `/v1/audio/transcriptions` with *“does not support audio
input”*; Kokoro is text-to-speech (`/v1/audio/speech`). For a server engine run a
Whisper server such as
`docker run -p 9001:8000 ghcr.io/speaches-ai/speaches`, then set the endpoint to
`http://host:9001/v1` and pick a `faster-whisper-*` model.

## Quiet speech

**Settings → General → Quiet speech** (off by default) is for whispering or
speaking softly. When it is on, Flowline measures how loud your speech is,
boosts the clip toward a normal level (up to +32 dB, never quieter, with soft
clipping so spikes cannot distort), accepts quieter clips than usual, and loosens
Whisper's voice detector from 0.5 to 0.3. The meter in the pill is drawn larger so
you can see soft speech register. Tested with speech attenuated to a peak of
0.009: normal mode treats that as silence and returns nothing, quiet mode boosts
it x40 and transcribes it. Room noise alone stays empty rather than inventing
words, because the Silero voice detector rejects it. Best in a quiet room; in a
noisy one leave it off. Try it from the command line with
`--transcribe clip.wav --local-whisper --quiet`.

## Tone by app

**Style → Apps** (on by default) makes Flowline choose the Style tab from the
app you dictate into: Mail and Outlook use *Email*, Slack and Teams use *Work
messages*, Messages, WhatsApp and Discord use *Personal messages*, and anything
else uses *Other*. In a browser it reads the focused window's title (for example
"Inbox - Gmail" or "Slack") only to choose, and stores nothing. Apps you have
dictated into are listed there, and you can pin any of them to a category. Turn
the switch off to use the *Email* style everywhere, as before.

## Transforms and Notetaker

Both use a **text model** — any OpenAI-compatible chat server, such as Ollama
(`http://127.0.0.1:11434/v1`). Set it on the **Transforms** page and press
*Detect models*; nothing is contacted until you do.

- **Transforms**: Polish, Prompt Engineer and up to eight custom prompts. Try
  them on pasted text, or turn on *Auto apply after dictation* to rewrite each
  dictation before it is pasted (it falls back to the text as dictated if the
  model fails or takes over 25 s).
- **Global shortcut**: with Transforms on and a text model set, highlight text in
  any app and press **⌥1** (Polish), **⌥2** (Prompt Engineer) or **⌥3–⌥9** (your
  own, in list order). Flowline copies the selection, rewrites it, and pastes it
  back over the selection, then restores your clipboard. Needs Accessibility. While
  Transforms is on, those Option+digit keys no longer type ¡™£ and so on.
- **Command mode**: turn it on in Transforms, select text in any app, **hold
  ⌥Space**, say what to do ("make this shorter", "translate to French", "fix the
  grammar"), then release. Your speech is transcribed by the same engine as
  dictation and the text model applies it to the selection, which is replaced in
  place. Esc cancels. While it is on, ⌥Space no longer types a non-breaking space.
- **Notetaker**: *New note* records the microphone, transcribes it in 25 s
  slices with your speech engine, and on *Stop* writes a summary with action
  items. Notes are stored in `notes.json` beside `state.json`; stops at 2 hours.
  Microphone only — system audio is not captured.
  **Meeting auto-detect** (off until you turn on *Offer a note when a meeting
  starts* on the Notetaker page): Flowline offers to start a note when a Zoom
  meeting opens, or when Teams, Webex, FaceTime, Slack, Discord or a browser is
  using the microphone for several seconds. It reads only which of those apps
  are running and whether the microphone is busy; no audio is touched until you
  answer *Start note*. *Not a meeting* silences that app (reset on the page).
  A Zoom note stops itself about 20 s after the meeting ends; others stop with
  *Stop recording*. Your own dictation never triggers it.

## My Voice

The **My Voice** page has two parts.

- **Learn from my corrections** (off until you turn it on). After Flowline
  pastes a dictation it watches that one text field for 20 seconds. If you fix a
  misheard word or phrase, the fix is added to your Dictionary as a *learned*
  entry, and a toast tells you. Needs Accessibility. It never reads password
  fields, keeps only the corrected word (not the text around it), and ignores
  rewrites, added or dropped letters ("plan" to "plans"), edits outside the
  pasted text, and everyday words like "their". Only fixes that look like a
  recognition error, similar spelling or a change of capitals, are learned.
  Remove any learned entry on the page or in the Dictionary.
- **Your voice in numbers**: dictations, words, speaking pace, words cleaned up,
  and the words you use most, all computed on this Mac from your history.

## Data

Everything is stored in `~/Library/Application Support/Flowline/state.json`
(dictionary, snippets, history, scratchpad, settings). The app starts empty; the
old demo rows are removed automatically on first load. **Settings → Data and
privacy → Delete all dictations** clears history.

## Menu bar and background

- The waveform menu-bar item has **Open Flowline** and **Quit Flowline**.
- Closing the hub hides it; Flowline keeps running so the shortcut still works.
  Quit with ⌘Q, the menu bar, or **Quit**.
- **Launch app at login** writes `~/Library/LaunchAgents/com.flowline.app.plist`
  for the current executable; **Show app in dock** switches between a regular
  and an accessory (menu-bar-only) app.
- Only one instance runs at a time, so the login item cannot fight the instance
  you already have open.

## Inspection flags

- `--screenshot <file.png>` — draw 120 frames, save the window, exit.
- `--engine-probe` — print the endpoint, selected model, server list and
  readiness, then exit.
- `--transcribe <file.wav> [--local-whisper] [--model <id>] [--endpoint <url>]`
  — transcribe a WAV and print the text.
- `--record-test` — record 1.2 s and report the WAV size.
- `--voice-test <secs>` — record, transcribe with the configured engine, print it.
- `--diagnose` — print version, permissions, frontmost app, clipboard and
  on-device engine readiness.
- `--transform <text>` / `--summarize <text>` / `--command <instruction> <text>` with `--text-endpoint <url> --text-model <id>` — run the text model once.
- `--transcribe <file.wav> … --quiet` — apply quiet-speech mode to that clip.
- `--hub` — start in the hub instead of the welcome screen.
- `--page home|dictionary|snippets|style|transforms|scratchpad|notetaker|insights|myvoice|settings`
- `--dark`, `--modal add-word|add-snippet|delete-word`,
  `--settings-tab general|system|data|account|about`, `--model-menu`

## Licence

Flowline is released under the [MIT licence](LICENSE). It bundles whisper.cpp,
also MIT licensed; see [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
