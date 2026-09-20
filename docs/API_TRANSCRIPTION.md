# API transcription

This fork adds a remote transcription engine without removing any of Handy's
local engines. The feature targets services that implement the OpenAI-compatible
`POST /audio/transcriptions` contract.

## When to use it

- Use a local model when audio privacy, offline operation or predictable cost is
  the priority.
- Use Groq when interactive turnaround and low local CPU/GPU usage are the
  priority. Actual latency depends on audio length, network path and provider
  load.
- Use a custom endpoint for a compatible self-hosted service or another
  provider. Remote custom endpoints must use HTTPS; plain HTTP is accepted only
  for loopback addresses such as `localhost` and `127.0.0.1`.

## Configure Groq

1. Install the API Edition from this fork's Releases page, or build and launch
   the `api-transcription` branch.
2. Open **Settings → Models → API transcription**.
3. Edit the **Groq** preset.
4. Enter your API key. The key field is intentionally blank when the dialog is
   reopened; Handy never sends the stored value back to the webview.
5. Keep `whisper-large-v3-turbo`, or choose another transcription model that
   Groq exposes through the same endpoint.
6. Save, click **Test**, then click **Use**.
7. Select **API Transcription** in the model list.

Selecting a fixed spoken language can avoid automatic language detection. For
the shortest start delay, open debug settings with `Ctrl+Shift+D` on Linux or
Windows (`Cmd+Shift+D` on macOS) and enable **Always-On Microphone**.

## Request contract

Handy sends a multipart form with:

- `file`: 16 kHz, mono, 16-bit WAV;
- `model`: the configured model name;
- `language`: the normalized language code when language forwarding is enabled;
- additional scalar fields from the optional JSON object.

`file`, `model` and `language` are reserved and cannot be overridden by extra
parameters. A successful endpoint must return UTF-8 JSON containing a string
field named `text`:

```json
{ "text": "transcribed text" }
```

Responses are limited to 1 MiB. Redirects are deliberately disabled so an API
key cannot be forwarded to a different destination. Retryable network, rate
limit and server errors use a short bounded retry sequence; timeouts are not
retried.

## Credential storage

Transcription API keys are not part of `settings_store.json` and are not
included in exported settings or this repository. The `keyring` backend uses:

- Secret Service on Linux;
- Keychain on macOS;
- Credential Manager on Windows.

Changing a custom endpoint's host or authentication scheme clears its stored
credential. Enter the key again to confirm that it may be sent to the new
destination. Preset host and authentication fields are locked; duplicate a
preset to create an editable custom profile.

## Privacy and cost

Remote mode sends each completed recording to the selected endpoint. Review the
provider's retention, privacy and billing policies before enabling it. Handy
does not log API keys. Production builds also redact transcription text from
application logs.

## Troubleshooting

- **No key:** edit the endpoint and save the key again.
- **Credential store unavailable:** ensure the desktop keyring is running and
  unlocked. On headless Linux, configure a Secret Service implementation.
- **HTTP rejected:** use HTTPS, or bind a local service to a loopback address.
- **Invalid JSON response:** confirm that the endpoint implements the response
  contract above instead of returning a web page or plain text.
- **Slow first beep:** enable Always-On Microphone. This trades a small amount of
  resource usage for lower capture startup latency.
- **Transcription works but paste fails:** use a standard paste method, or see
  the optional [X11 smart paste helper](../extras/smart-paste-x11/README.md).
