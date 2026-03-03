# TODO: Apple Developer ID Certificate Signing

**Priority:** Medium (improves UX but not blocking)
**Effort:** Low (~1 hour setup, ongoing $99/year)
**Impact:** Eliminates macOS Keychain popup for "Always Allow"

## Problem

On macOS, unsigned binaries trigger a Keychain access popup every time they access stored secrets. Users must click "Always Allow" but this button **does not persist** for ad-hoc signed binaries.

```
┌──────────────────────────────────────────────────┐
│  "spn" wants to use your confidential            │
│  information stored in "spn:anthropic" in        │
│  your keychain.                                  │
│                                                  │
│  ┌──────────┐  ┌─────────┐  ┌──────────────┐    │
│  │  Deny    │  │ Allow   │  │ Always Allow │    │
│  └──────────┘  └─────────┘  └──────────────┘    │
└──────────────────────────────────────────────────┘
        ↑ This works but only with Developer ID
```

## Solution

1. **Join Apple Developer Program** ($99/year)
   - https://developer.apple.com/programs/

2. **Create Developer ID Application certificate**
   - Keychain Access → Certificate Assistant → Request Certificate
   - Download from Apple Developer portal

3. **Sign the binary**
   ```bash
   codesign --sign "Developer ID Application: SuperNovae Studio (XXXXXXXXXX)" \
            --options runtime \
            --timestamp \
            target/release/spn
   ```

4. **Notarize for Gatekeeper** (optional but recommended)
   ```bash
   xcrun notarytool submit spn.zip --apple-id X --team-id Y --password Z
   ```

5. **Update CI/CD**
   - Store certificate in GitHub Secrets
   - Sign during release workflow
   - Notarize macOS builds

## Workaround (Current)

Users can still use spn with Keychain - they just get a popup on each access until they:
1. Click "Always Allow" (doesn't persist for unsigned)
2. Or use environment variables instead (less secure but no popup)

## References

- [Apple Developer ID](https://developer.apple.com/developer-id/)
- [Code Signing Guide](https://developer.apple.com/documentation/security/code_signing_services)
- [Notarization](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)

## Decision

- [ ] Purchase Apple Developer Program membership
- [ ] Set up code signing in CI
- [ ] Update Homebrew formula with signed binary
- [ ] Document in README
