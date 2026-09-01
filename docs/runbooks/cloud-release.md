# KeyhanStudio Cloud release runbook

The AudioMaster API service lives in the standalone `keyhan-studio-core` repository. AudioMaster must never be deployed with a cloud origin other than HTTPS (localhost is allowed for development).

## Deployment gate

1. Back up the production SQLite database and verify restore procedures.
2. Deploy the database initialization changes and `/audiomaster` router to staging.
3. Run `npm test` in the API repository.
4. Run `node scripts/cloud-smoke.mjs` with `KEYHAN_CLOUD_URL` and a staging access token. Set `KEYHAN_CLOUD_MUTATION_TEST=1` only on a disposable staging account.
5. Verify two-device conflict handling, rotating refresh-token reuse rejection, offline startup, `429 Retry-After`, and network loss during sync.
6. Confirm request logs contain no settings bodies, tokens, audio fields, or local paths.
7. Deploy one instance, exercise sync and feedback, then roll through the fleet. Roll back the application before restoring the database if errors increase.

Production deployment, DNS, TLS, and token-rotation verification require KeyhanStudio infrastructure credentials and cannot be performed from a source checkout.
