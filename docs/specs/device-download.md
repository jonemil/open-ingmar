# Device download protocol

This is an observed private protocol, not a Garmin-published API contract.
Authentication and authenticated downloads have not yet been tested here.
This describes the SDK support script, separate from the compiler. Its protocol
was documented from local SDK-manager 1.0.16 inspection and the independent
[reference CLI][reference-cli] at revision
`ed5d91fbacc05ad561aa448640a539a73b6c6ace` (Apache-2.0). Relevant reference files
are `internal/oauth/oauth.go`, `internal/client/login.go`,
`internal/client/devices.go`, and `internal/client/fonts.go`. The script was
written independently; the manager investigation is retained only locally.

## Browser login

Start a callback listener bound to loopback. Send the user to Garmin's SSO
sign-in page at `https://sso.garmin.com/sso/signin`, with the web client
identifier `ConnectIqSdkManager`, and set `service`, `source`,
`redirectAfterAccountLoginUrl`, and `redirectAfterAccountCreationUrl` to the
same local callback URL. An independently implemented CLI demonstrates this
callback mechanism.

Receive a nonempty `ticket` query parameter after the browser authenticates. The
service URL supplied during token exchange must match the callback URL used for
login. Keep callback tickets out of HTTP request logs.

## Token exchange

POST a URL-encoded form to:

```text
https://mobile-auth.security.services.garmin.com/oauth/service_ticket/token
```

| Field | Value |
|---|---|
| `grant_type` | `service_ticket` |
| `client_id` | `CIQ_SDK_MANAGER` |
| `service_ticket` | Ticket received from the browser callback |
| `service_url` | Exact callback URL |

Use the returned `access_token` as `Authorization: Bearer <token>` on GCS
requests. Our one-shot script retains tokens only in memory and signs in again
on each invocation; it does not implement refresh-token persistence.

The independently implemented CLI uses the older
`https://services.garmin.com/api/oauth/token` endpoint. Our downloaded official
manager 1.0.16 uses the mobile-auth endpoint above. Interoperation between that
endpoint and a standalone browser callback remains to be verified in a live
session.

## Catalogue and device package

GET:

```text
https://api.gcs.garmin.com/ciq-product-onboarding/devices?sdkManagerVersion=1.0.16
```

The expected response is an array of device records. Select by `name`, such as
`fr265`; filter out records explicitly marking `ciqInfoFileExists` false. When
multiple records match, prefer the newest `lastUpdateTime`. Its representation
has not been confirmed live; the current script assumes chronologically sortable
timestamp strings.

The package endpoint takes a **hardware part number**, not the compiler device
name:

```text
https://api.gcs.garmin.com/ciq-product-onboarding/devices/{partNumber}/ciqInfo
```

Catalogue variants observed in reference implementations expose either
`partNumber` or a `partNumbers` array. Manager 1.0.16 chooses the first element
of its internal part-number list for the package request. URL-encode the
selected number as one path component.

Expect a ZIP package. Install its contents under `Devices/{name}/` within the
Garmin data root. The script requires root-level `compiler.json` and
`simulator.json` before publishing a newly extracted package and preserves
existing installations.

## Simulator fonts

`simulator.json` describes fonts as groups under `fonts`, each containing
another `fonts` array with `filename` fields. For each downloadable filename,
GET:

```text
https://api.gcs.garmin.com/ciq-product-onboarding/fonts/font?fontName={filename}
```

The expected response is a ZIP to extract under `Fonts/`. A `.md5` file records
the archive digest as an SDK-manager compatibility marker; it is not
cryptographic authenticity verification. Font descriptions containing spaces are
skipped, following observed reference behavior. Unexpected archives and HTTP
failures stop the script; device installation may already have completed if a
later font request fails.

[reference-cli]: https://github.com/lindell/connect-iq-sdk-manager-cli
