START TRANSACTION;
ALTER TABLE backuprepositories ADD controlstartedat bigint;

ALTER TABLE backuprepositories ADD controlstate text NOT NULL DEFAULT 'Idle';

ALTER TABLE backuprepositories ADD currentrunid uuid;

ALTER TABLE backuppolicies ADD controlstartedat bigint;

ALTER TABLE actions ADD controlstartedat bigint;

UPDATE actions SET controlstartedat = NULL
WHERE id = '41000000-0000-0000-0000-000000000001';

UPDATE actions SET controlstartedat = NULL
WHERE id = '41000000-0000-0000-0000-000000000002';

CREATE INDEX ix_backuprepositories_controlstate_controlstartedat ON backuprepositories (controlstate, controlstartedat);

CREATE INDEX ix_backuppolicies_controlstate_controlstartedat ON backuppolicies (controlstate, controlstartedat);

CREATE INDEX ix_actions_controlstate_controlstartedat ON actions (controlstate, controlstartedat);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260714182009_migration0006', '10.0.9');

COMMIT;

