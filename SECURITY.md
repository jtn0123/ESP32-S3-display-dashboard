# Security Policy

## Supported Versions

Currently supporting security updates for:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly:

1. **DO NOT** create a public GitHub issue
2. Email the details to: [your-security-email@example.com]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Fix Timeline**: Depends on severity
  - Critical: 1-2 weeks
  - High: 2-4 weeks
  - Medium/Low: Next release cycle

## Security Best Practices

This project implements:

- Exact dependency version pinning
- Automated security audits via `cargo audit`
- Regular dependency updates via Dependabot
- Secure TLS/HTTPS communications
- No hardcoded credentials or secrets

## Known Security Considerations

- ESP32-S3 devices support secure boot and flash encryption (not enabled by default)
- OTA updates should verify signatures before applying
- Wi-Fi credentials are stored in NVS (consider encryption for production)