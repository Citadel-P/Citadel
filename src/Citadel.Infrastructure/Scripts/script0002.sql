START TRANSACTION;
ALTER TABLE backuprunitems DROP CONSTRAINT fk_backuprunitems_platforms_platformid;

ALTER TABLE backuprunitems ADD CONSTRAINT fk_backuprunitems_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE;

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260715212312_migration0002', '10.0.9');

COMMIT;

