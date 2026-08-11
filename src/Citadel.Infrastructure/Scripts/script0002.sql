START TRANSACTION;
CREATE TABLE swarmservicestats (
    id uuid NOT NULL,
    cpuusage double precision NOT NULL,
    created bigint NOT NULL,
    dockerserviceid text NOT NULL,
    dockertaskid text NOT NULL,
    memoryactive double precision NOT NULL,
    memorycache double precision NOT NULL,
    memorylimit double precision NOT NULL,
    platformid uuid NOT NULL,
    rxbytes double precision NOT NULL,
    servicename text NOT NULL,
    stackid uuid,
    swarmserviceid uuid,
    taskkey text NOT NULL,
    txbytes double precision NOT NULL,
    CONSTRAINT pk_swarmservicestats PRIMARY KEY (id),
    CONSTRAINT fk_swarmservicestats_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE INDEX ix_swarmservicestats_created ON swarmservicestats (created);

CREATE INDEX ix_swarmservicestats_managedservicecreated ON swarmservicestats (swarmserviceid, created) WHERE swarmserviceid IS NOT NULL;

CREATE INDEX ix_swarmservicestats_platformservicecreated ON swarmservicestats (platformid, dockerserviceid, created);

CREATE UNIQUE INDEX ix_swarmservicestats_platformtaskcreated ON swarmservicestats (platformid, dockertaskid, created);

CREATE INDEX ix_swarmservicestats_stackservicecreated ON swarmservicestats (stackid, servicename, created) WHERE stackid IS NOT NULL;

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260811083833_migration0002', '10.0.10');

COMMIT;

