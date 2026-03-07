# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.14.x  | :white_check_mark: |
| 0.12.x  | :white_check_mark: |
| < 0.12  | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability, please follow these steps:

### Do NOT

- Open a public GitHub issue
- Disclose the vulnerability publicly before it's fixed

### Do

1. **Email us directly** at: `security@supernovae.studio`
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes (optional)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: 24-72 hours
  - High: 7 days
  - Medium: 30 days
  - Low: Next release

### Security Features

SuperNovae CLI implements several security measures:

- **OS Keychain Integration**: Secrets stored in macOS Keychain, Windows Credential Manager, or Linux Secret Service
- **Memory Protection**: `mlock()` prevents secrets from being swapped to disk
- **Core Dump Protection**: `MADV_DONTDUMP` excludes secrets from core dumps
- **Automatic Zeroization**: `Zeroizing<T>` clears secrets on drop
- **Socket Permissions**: Daemon socket is `0600` (owner-only)
- **Peer Verification**: `SO_PEERCRED` / `LOCAL_PEERCRED` for IPC authentication

### Scope

In scope:
- Secret/credential handling
- Daemon IPC security
- Path traversal vulnerabilities
- Code injection vulnerabilities
- Authentication/authorization issues

Out of scope:
- Denial of service (unless severe)
- Issues requiring physical access
- Social engineering attacks

## Acknowledgments

We thank all security researchers who responsibly disclose vulnerabilities. With your permission, we'll acknowledge you in our release notes.
