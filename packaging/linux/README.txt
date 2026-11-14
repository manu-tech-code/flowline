Flowline for Linux
==================

0. (Optional) Install it for your user, with a menu entry and icon:

       sh install.sh

1. Run the one-time setup, then log out and back in:

       sh flowline-setup.sh

   (It lets Flowline read your keyboard for the global shortcut and press
   Ctrl+V for you. Without it the app still works, but only inside its window,
   and dictated text is left on the clipboard instead of being pasted.)

2. Start Flowline:

       ./flowline

3. Open Settings > Transcription engine and press "Download Small". The speech
   model (about 466 MB) is fetched once from huggingface.co. Audio never leaves
   your computer.

Shortcuts
---------
  Hold Right Ctrl         dictate anywhere (change it in Settings)
  Ctrl+Alt+Shift+1..9     run a Transform on the selected text
  Hold Ctrl+Alt+Space     command mode: speak an edit for the selected text
  Esc                     cancel
  Ctrl+Q                  quit

Closing the window keeps Flowline running in the background. Start it again
(from your applications menu, or ./flowline) to bring the window back.

What you need
-------------
  - A desktop with PipeWire or PulseAudio (any current distribution).
  - For meeting detection and "mute other audio": the `pactl` command, which
    comes with PipeWire/PulseAudio.
  - Optional: zenity or kdialog (the "meeting detected" prompt and the file
    picker), and xdotool or wtype if you cannot use the uinput setup.

Licence: MIT. See LICENSE and THIRD-PARTY-NOTICES.md.
