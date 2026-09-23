# Vozel security policy

## Supported code

Security fixes for Vozel are maintained on this repository's default
publication branch. This independent fork is not an upstream Handy support
channel; upstream-only issues belong in the upstream project.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting for this repository when it is
available. Do not place API keys, recordings, transcripts, credential-store
exports or other sensitive data in a public issue.

Include:

- the affected commit and operating system;
- a minimal reproduction without real credentials;
- the expected and observed behavior;
- the security impact;
- logs with secrets and transcription contents removed.

For ordinary, non-sensitive bugs, open a public issue and state that the problem
affects Vozel.

## Credential design

Transcription provider keys are stored in the native operating-system
credential store and are not serialized with application settings. Authenticated
requests reject remote plain HTTP endpoints, do not follow redirects and clear a
stored key when a custom endpoint's destination or authentication scheme changes.

Local transcription does not send audio to a remote provider. API transcription
sends the completed recording to the endpoint explicitly selected by the user.
