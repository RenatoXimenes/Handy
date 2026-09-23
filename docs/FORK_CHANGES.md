# Vozel fork changes

This document records changes maintained by Vozel on top of upstream
[cjpais/Handy](https://github.com/cjpais/Handy). Vozel is independently
maintained and is not affiliated with or endorsed by upstream. The publication
branch is based on upstream `main` after the v0.9.7 release.

The repository URL remains
[RenatoXimenes/Handy](https://github.com/RenatoXimenes/Handy) until its rename
is approved. On first launch, Vozel copies settings, models, recordings and
history from the legacy Handy data location without deleting the originals.

## Vozel identity and local model

- Uses a separate app identifier, original icons and its own support links.
- Offers the Ottema AI Nemotron 3.5 ASR PT-BR fine-tune as an optional,
  SHA-256-verified GGUF download. The model is subject to OpenMDW-1.1.
- Preserves upstream attribution and historical release provenance.

## Remote transcription

- Adds an `API Transcription` engine alongside every existing local engine.
- Includes locked presets for Groq and OpenAI.
- Supports custom OpenAI-compatible transcription endpoints.
- Sends 16 kHz mono WAV, model, optional language and validated extra fields.
- Provides endpoint creation, duplication, activation and connection testing.
- Uses `whisper-large-v3-turbo` in the Groq preset.

## Security and privacy

- Stores keys in the native operating-system credential store.
- Never serializes transcription keys into Handy settings.
- Requires HTTPS except for local loopback development endpoints.
- Disables HTTP redirects on authenticated transcription requests.
- Clears a custom endpoint's key when its destination or authentication scheme
  changes.
- Limits response bodies and accepts only the documented JSON schema.
- Keeps local transcription available for fully offline use.
- Disables automatic updates until the fork has its own signed release feed;
  using upstream releases would remove the fork-specific features.

## Linux input improvements

- Falls back to non-blocking shortcut capture when exclusive capture is not
  available.
- Uses a 1 ms delay for direct `xdotool` typing.
- Provides an optional, separately installed X11 smart-paste helper for terminal
  and graphical application workflows.

## Deliberate non-goals

- This fork does not claim that cloud transcription is more private than local
  transcription.
- It does not embed, export or back up API keys.
- It does not claim one accuracy or speed score for every remote provider and
  model.
- The X11 smart-paste helper is not presented as a universal native-Wayland
  solution.

## Verification policy

Before publication, changes are checked with Rust tests, frontend build/lint,
format checks, a tracked-file and Git-history secret scan, and targeted tests for
URL validation, redirects, response schema and response-size limits. Platform
claims that are not covered by CI or a smoke test must remain explicitly marked
as unverified.

The new interface is authored in English and Portuguese. Other locale files
contain the English source text as an explicit fallback so the interface and
translation-key checks remain complete; native translations are welcome.
