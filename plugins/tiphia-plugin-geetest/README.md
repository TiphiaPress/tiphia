# Tiphia GeeTest Plugin

`tiphia-geetest` adds GeeTest v4 captcha verification to administrator login,
public registration, and blog comments.

## Configuration

Open the admin plugin page, choose `tiphia-geetest`, and fill:

```json
{
  "captcha_id": "your-geetest-captcha-id",
  "captcha_key": "your-geetest-captcha-key",
  "verify_login": true,
  "verify_register": true,
  "verify_comment": true
}
```

- `captcha_id`: public GeeTest v4 captcha ID used by the frontend widget.
- `captcha_key`: private GeeTest v4 key used by the server to validate proofs.
- `verify_login`: require captcha for admin login.
- `verify_register`: require captcha for public registration.
- `verify_comment`: require captcha for public blog comments and replies.

The plugin state is disabled by default after installation. Enable it from the
admin plugin page when you are ready to use it.

When either `captcha_id` or `captcha_key` is empty, the plugin stays installed
but captcha verification is disabled even if the plugin state is enabled. The
public frontend can detect this with:

```http
GET /api/v1/geetest/config
```

Example response:

```json
{
  "enabled": true,
  "captcha_id": "your-geetest-captcha-id",
  "verify_login": true,
  "verify_register": true,
  "verify_comment": true
}
```

The frontend must submit the GeeTest validation object as `captcha` in login,
registration, or comment requests:

```json
{
  "account": "root",
  "password": "your-password",
  "captcha": {
    "lot_number": "...",
    "captcha_output": "...",
    "pass_token": "...",
    "gen_time": "..."
  }
}
```

The server signs `lot_number` with `captcha_key` and verifies the proof against
GeeTest's v4 validation endpoint.

Reference: <https://docs.geetest.com/BehaviorVerification/apirefer/api/server>
