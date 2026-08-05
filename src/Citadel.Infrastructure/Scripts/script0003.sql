START TRANSACTION;
ALTER TABLE swarmserviceprojections ADD dockerstacknamespace text;

ALTER TABLE swarmserviceprojections ADD ownership text NOT NULL DEFAULT 'Unmanaged';

ALTER TABLE swarmserviceprojections ADD ownershipdiagnostic text;

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260804150141_migration0003', '10.0.10');

COMMIT;
