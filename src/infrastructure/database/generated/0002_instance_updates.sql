-- Append-only upgrade from the published initial catalog.

-- Instance update state is independent of Activity retention.
CREATE TABLE instanceupdatechecks (
    id integer NOT NULL PRIMARY KEY CHECK (id = 1),
    lastattempt timestamp with time zone,
    lastsuccess timestamp with time zone,
    error text,
    available jsonb
);

CREATE TABLE instanceupdatediscoveries (
    instanceid uuid NOT NULL,
    version text NOT NULL,
    discoveredat timestamp with time zone NOT NULL,
    PRIMARY KEY (instanceid, version)
);

CREATE TABLE instanceupdateplans (
    id uuid NOT NULL PRIMARY KEY,
    actorid uuid NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    plan jsonb NOT NULL
);

CREATE TABLE instanceupdates (
    id uuid NOT NULL PRIMARY KEY,
    instanceid uuid NOT NULL,
    actorid uuid NOT NULL,
    idempotencykey uuid NOT NULL,
    ownsinstallation boolean NOT NULL DEFAULT TRUE,
    revision bigint NOT NULL CHECK (revision >= 0),
    operation jsonb NOT NULL,
    completion jsonb,
    UNIQUE (instanceid, idempotencykey)
);
CREATE UNIQUE INDEX ix_instanceupdates_owner ON instanceupdates(instanceid) WHERE ownsinstallation;

CREATE TABLE instanceupdateeventreceipts (
    operationid uuid NOT NULL REFERENCES instanceupdates(id),
    eventkey text NOT NULL,
    PRIMARY KEY (operationid, eventkey)
);

ALTER TABLE instancesetupstates
    ADD COLUMN updatecheckintervalhours integer NOT NULL DEFAULT 6,
    ADD CONSTRAINT ck_instancesetupstates_updatecheckintervalhours
        CHECK (updatecheckintervalhours IN (6, 12, 24));

-- Observed through signed build-pool health checks, fenced by pool configuration.
ALTER TABLE buildagentpools ADD COLUMN agentversion text;
