START TRANSACTION;
CREATE TABLE backuprunitems (
    id uuid NOT NULL,
    backuprunid uuid NOT NULL,
    bytesadded bigint,
    bytesprocessed bigint,
    completedat timestamp with time zone,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    errorcode text,
    errormessage text,
    exitcode integer,
    filesprocessed bigint,
    parentsnapshotid text,
    platformid uuid NOT NULL,
    resticsnapshotid text,
    startedat timestamp with time zone,
    status text NOT NULL,
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    volumename text NOT NULL,
    CONSTRAINT pk_backuprunitems PRIMARY KEY (id),
    CONSTRAINT fk_backuprunitems_backupruns_backuprunid FOREIGN KEY (backuprunid) REFERENCES backupruns (id) ON DELETE CASCADE,
    CONSTRAINT fk_backuprunitems_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT
);

CREATE TABLE stackreleasevolumebindings (
    id uuid NOT NULL,
    composevolumename text,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    isanonymous boolean NOT NULL DEFAULT FALSE,
    isexternal boolean NOT NULL DEFAULT FALSE,
    platformid uuid NOT NULL,
    stackreleaseid uuid NOT NULL,
    volumename text NOT NULL,
    CONSTRAINT pk_stackreleasevolumebindings PRIMARY KEY (id),
    CONSTRAINT fk_stackreleasevolumebindings_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT,
    CONSTRAINT fk_stackreleasevolumebindings_stackreleases_stackreleaseid FOREIGN KEY (stackreleaseid) REFERENCES stackreleases (id) ON DELETE CASCADE
);

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('677df0f0-2ce5-4b25-76ad-eb4e21f0748d', 4, 17, '30000000-0000-0000-0000-000000000001', 768);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('9042fcd7-44f2-8a16-dca9-2fabdba5a0bc', 1, 17, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('f2552404-ef60-5f22-0eaf-fd7db21f2579', 2, 17, '30000000-0000-0000-0000-000000000002', 768);

CREATE INDEX ix_backuprunitems_platform_volumename ON backuprunitems (platformid, volumename);

CREATE INDEX ix_backuprunitems_resticsnapshotid ON backuprunitems (resticsnapshotid);

CREATE INDEX ix_backuprunitems_run_status ON backuprunitems (backuprunid, status);

CREATE INDEX ix_backuprunitems_run_volumename ON backuprunitems (backuprunid, volumename);

CREATE INDEX ix_stackreleasevolumebindings_platform_volumename ON stackreleasevolumebindings (platformid, volumename);

CREATE UNIQUE INDEX ix_stackreleasevolumebindings_release_volumename ON stackreleasevolumebindings (stackreleaseid, volumename);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260714103514_migration0004', '10.0.9');

COMMIT;

