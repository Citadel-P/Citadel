START TRANSACTION;
ALTER TABLE backuppolicies ADD webhook jsonb;

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260714141353_migration0005', '10.0.9');

COMMIT;

