# OpenAI 관리자 키 보안 저장소

TokenGlass stores the OpenAI organization administrator API key in the current user's
operating system credential store. The key is not returned to the WebView and is not
stored in the general `settings.json` after migration.

## Platform behavior

| Platform | Credential store | Previous general settings file |
| --- | --- | --- |
| macOS | Login Keychain | `~/Library/Application Support/com.mykim.tokenglass/settings.json` |
| Windows | Windows Credential Manager | Tauri app data directory's `settings.json` |
| Linux | Secret Service (through the user's D-Bus session) | Tauri app data directory's `settings.json` |

The previous file held the key as ordinary JSON text. File permissions and disk
encryption may reduce exposure, but they do not provide the same per-credential
protection as the platform credential store. On first launch after upgrade, TokenGlass
copies the legacy value to the credential store, reads it back to verify it, and then
removes the legacy setting. A failed write or verification leaves the legacy value in
place so it can be retried.

TokenGlass uses one Rust credential-store interface backed by each platform's native
user store. This avoids introducing a separate vault password and keeps credential
access inside the backend.

## Linux requirements

Linux requires an available Secret Service implementation and a working user D-Bus
session. Headless sessions, locked services, or systems without a Secret Service may
report that the credential store is unavailable or access was denied. In that case,
start or unlock the desktop keyring service and retry. TokenGlass does not save the key
to a plain file as a fallback.

## Removing the key

Use **Settings → OpenAI key → Delete key** in TokenGlass. This removes the credential
from the OS store and clears the in-memory usage cache. Removing the application does
not necessarily remove credentials from the OS store; if needed, remove the
`com.mykim.tokenglass` / `openai-admin-key` credential using the platform's credential
manager (Keychain Access on macOS, Credential Manager on Windows, or the desktop
keyring manager on Linux).

## Downgrading to an older app version

Older versions do not read the OS credential store. Do not copy the key back to
`settings.json`. After downgrading, enter the key again in the older version if that
version is still required. The newer version will migrate a legacy settings value on
next launch, but a credential stored only in the OS store is intentionally not
exported to plaintext during rollback.

## Scope

This protects the key at rest through the operating system and keeps it out of normal
frontend state and diagnostics. The key must exist in process memory while the backend
makes an OpenAI request. This does not protect against malicious code running as the
same user or memory inspection of the running process.
