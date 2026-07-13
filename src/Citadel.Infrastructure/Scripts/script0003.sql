START TRANSACTION;
CREATE INDEX ix_backuppolicies_source_gin ON backuppolicies USING gin (source);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260713161455_migration0003', '10.0.9');

COMMIT;

