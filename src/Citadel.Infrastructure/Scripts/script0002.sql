START TRANSACTION;
ALTER TABLE platformstats ADD disktotalbytes bigint;

ALTER TABLE platformstats ADD diskusage double precision;

ALTER TABLE platformstats ADD diskusedbytes bigint;

INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000023', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Disk > 70% - Platform', '[]', 3, 'Warning', 70.0, 'PlatformDiskHigh');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000024', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Disk > 90% - Platform', '[]', 3, 'Critical', 90.0, 'PlatformDiskHigh');

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260726191424_migration0002', '10.0.10');

COMMIT;

