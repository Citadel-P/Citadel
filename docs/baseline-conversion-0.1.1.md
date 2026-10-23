# Demo and preview baseline conversion for 0.1.1

This is an explicit, offline maintenance procedure for the two development
installations. It retains a single `0001` baseline. Ordinary deployments do not
perform data conversion or change existing migration journals.

The supported source baseline checksum is
`c622e2f16a0b11b3ff2a6021c1f097f7b97cd575fdfb241a63466ef1fc444d05`.
The target checksum is
`75c219301520bcb5c5a9904526334a5fe5f45cb57bc554c56062d3b185b4afa0`.
The sole schema difference is `alertrulestates.lastobservedat`.

## Prepare and rehearse

1. Record the source release, image digests, migration journal, Compose project,
   rootless Docker engine identity and mounts for each environment independently.
2. Export a PostgreSQL custom-format dump. Back up Core data, signing/encryption
   keys, private environment files, Compose overlays and deployment state. Keep
   all backups and conversion reports outside the repository, with restrictive
   permissions and a recovery copy outside the VPS.
3. Restore the dump into a disposable database in an isolated PostgreSQL 18
   container. Do not run Core workers against the restored configuration.
4. Create another empty database in that container. Initialize it using the new
   Core's `migrate` command. This writes the new baseline journal normally.
5. Import with explicit database names:

```bash
python3 src/tools/release/baseline_import.py \
  --container <isolated-postgres-container> \
  --source-database <restored-source-copy> \
  --target-database <fresh-initialized-target> \
  --report /private/maintenance/preview-conversion.json
```

The tool refuses the container's default installation database, unsupported
journals and any schema difference beyond the expected column. It replaces all
target resource data, including initial seeds, with source records in one
transaction. IDs, customized rules and encrypted values are preserved. The new
observation timestamp is initialized from `lasttriggeredat`. Neither journal is
rewritten.

The private import session temporarily suppresses triggers to restore cyclic
relationships without generating synthetic history. Before commit it re-enables
triggers, validates every foreign key and compares row counts and sorted content
checksums for every source table. Failure rolls back the imported records; discard
the disposable target after a failure, since PostgreSQL sequence changes need not
roll back. Database diagnostics containing private records are not printed.

## Cut over each environment

1. Prevent automated deployment during maintenance. Wait for active operations,
   acquire the environment's deployment lock and stop Core. Keep workloads and
   the original PostgreSQL storage intact.
2. Capture final consistent backups, including Core data and deployment files.
   Repeat the tested conversion using this final database snapshot.
3. Export the converted database, restore it into new target database storage,
   and retain the corresponding Core data and original recovery keys. Verify the
   new journal and run the new Core's `migrate` command again: zero migrations
   should be applied.
4. Point the existing deployment at the converted storage and new immutable Core
   and paired Agent references. Preserve rootless identity, project/network
   identity, ports, keys, and publication/deployment high-water safeguards.
5. Start Core and verify readiness, administrator login, stored credential
   decryption, Platform/Agent connections, resource counts, alerts, jobs and live
   observations. Perform preview before demo.
6. Retain old storage and the recorded previous release until verification and
   the agreed retention period finish. Restore the complete previous installation
   through an explicit operator recovery if needed; changing the image alone does
   not restore its database. Re-enable deployment only after the new database is
   compatible with the release that automation will select.

Do not delete original volumes, rewrite an applied checksum, point Core at a
rehearsal copy, or allow both old and new Core instances to operate simultaneously.

## Verification

```bash
CITADEL_BASELINE_IMPORT_CONTAINER=<disposable-postgres-container> \
  python3 -m unittest discover -s test/release -p test_baseline_import.py -v
```

The fixture tests successful data preservation, rollback on invalid foreign keys,
schema drift rejection and identifier validation. CI runs them in its disposable
database service. Real environment dumps remain private and are never CI fixtures.
