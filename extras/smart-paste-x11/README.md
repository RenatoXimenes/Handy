# Smart paste helper for X11

Handy normally needs one configured paste shortcut. Graphical applications such
as browsers, web chats, editors and ordinary text fields generally use
`Ctrl+V`, while terminal emulators generally use `Ctrl+Shift+V`. This optional
helper inspects the active window and chooses the appropriate shortcut.

It is useful for workflows that alternate between:

- browser text fields and chat applications;
- desktop editors and forms;
- GNOME Terminal, Console/KGX, Konsole, Tilix, Terminator, XFCE Terminal,
  Alacritty, Kitty, WezTerm, XTerm, URxvt and Foot.

## Scope and limitations

The helper uses X11 window metadata (`WM_CLASS`). It is intended for X11 and
applications running through XWayland. Native Wayland applications may not
expose enough information to `xdotool`/`xprop`; use Handy's built-in Wayland
paste tools in that case.

Window-class detection is necessarily heuristic. An application that embeds
both a terminal and a normal editor under the same window class may need a
manual override.

## Install on Ubuntu/Debian

```bash
sudo apt install xdotool x11-utils xsel coreutils
./extras/smart-paste-x11/install.sh
```

Then open Handy and configure:

1. **Paste method:** `External script`.
2. **Script path:** `~/.local/bin/handy-smart-paste` (expand `~` to your home
   directory if the file picker requires an absolute path).
3. **Clipboard handling:** leave the clipboard available to the external
   script.

The installer changes no Handy settings and stores no microphone name, API key
or user-specific path in the repository.

## Manual override

The default mode is automatic. To force a behavior for troubleshooting, launch
Handy with one of these environment variables:

```bash
HANDY_PASTE_MODE=terminal handy   # always Ctrl+Shift+V
HANDY_PASTE_MODE=standard handy   # always Ctrl+V
```

Accepted values are `auto`, `terminal` and `standard`.

## Security notes

- The transcribed text is passed to the script as one argument by Handy and is
  written to the X11 clipboard without shell evaluation.
- X11 clipboard contents are visible to applications in the same graphical
  session. This is an X11 platform limitation, not API-key storage.
- The script never reads or handles transcription provider credentials.
