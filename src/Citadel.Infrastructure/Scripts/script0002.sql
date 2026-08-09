START TRANSACTION;
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, status, threshold, type)
VALUES ('019d0000-0001-7000-8001-00000000001c', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Operation Failed - Swarm Service', '[]', NULL, 'Critical', 'Enabled', NULL, 'SwarmServiceOperationFailed');

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260809164935_migration0002', '10.0.10');

COMMIT;

