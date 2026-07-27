START TRANSACTION;
CREATE INDEX ix_platformstats_created ON platformstats (created);

CREATE INDEX ix_containerstats_created ON containerstats (created);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260727074557_migration0003', '10.0.10');

COMMIT;

