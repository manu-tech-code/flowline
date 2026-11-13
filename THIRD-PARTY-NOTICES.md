# Third-party notices

Flowline is MIT licensed (see LICENSE). A release build also ships, or downloads
on request, the following third-party software.

## whisper.cpp (bundled: Contents/Resources/bin/whisper-cli)

The on-device speech engine is built from https://github.com/ggml-org/whisper.cpp
(see `scripts/build-whisper.sh`).

    MIT License
    
    Copyright (c) 2023-2026 The ggml authors
    
    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:
    
    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.
    
    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.

## Speech models (downloaded on request, not bundled)

The Whisper models offered in Settings are OpenAI's Whisper weights converted for
whisper.cpp (https://huggingface.co/ggerganov/whisper.cpp), released under the MIT
licence. The optional voice-detection model is Silero VAD
(https://huggingface.co/ggml-org/whisper-vad), also MIT licensed.

## Rust crates

Flowline is built with eframe/egui and other open-source Rust crates, each under
its own permissive licence (MIT and/or Apache-2.0). `cargo tree` lists them and
their licences are in each crate's repository.
