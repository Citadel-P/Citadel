START TRANSACTION;
ALTER TABLE platforms ADD description text;

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260710121759_migration0002', '10.0.9');

COMMIT;

