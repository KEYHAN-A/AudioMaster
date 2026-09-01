# Desktop signing and update runbook

Public builds are release candidates until every artifact is signed and independently installed on a clean machine.

- macOS: import the Developer ID Application certificate in CI, build the universal app, notarize with App Store Connect credentials, staple the ticket, and verify with `codesign --verify --deep --strict` and `spctl --assess --type execute`.
- Windows: sign the MSI/NSIS artifacts with the organization EV/OV certificate and RFC 3161 timestamp, then verify with `signtool verify /pa /all` on a clean Windows runner.
- Updater: create the Tauri updater signing key offline, store only the private key in CI secrets, commit the public key after security review, sign `latest.json` and every update archive, and test upgrade plus rollback from the prior public version.
- Generate checksums, CycloneDX SBOMs, and third-party notices. Attach them to the draft release before approval.

Certificate subjects, updater public key, support owner, and rollback release are release-specific evidence and must not be fabricated in source control.
