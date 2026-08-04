START TRANSACTION;
CREATE TABLE swarmconfigprojections (
    platformid uuid NOT NULL,
    dockerconfigid text NOT NULL,
    dockercreatedat timestamp with time zone,
    dockerupdatedat timestamp with time zone,
    isstale boolean NOT NULL DEFAULT FALSE,
    labels jsonb NOT NULL,
    name text NOT NULL,
    observedat timestamp with time zone NOT NULL,
    servicenames jsonb NOT NULL,
    templatingdriver text,
    versionindex bigint NOT NULL,
    CONSTRAINT pk_swarmconfigprojections PRIMARY KEY (platformid, dockerconfigid),
    CONSTRAINT fk_swarmconfigprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE swarmnetworkprojections (
    platformid uuid NOT NULL,
    dockernetworkid text NOT NULL,
    dockercreatedat timestamp with time zone,
    driver text NOT NULL,
    enableipv6 boolean NOT NULL,
    isattachable boolean NOT NULL,
    isencrypted boolean NOT NULL,
    isingress boolean NOT NULL,
    isinternal boolean NOT NULL,
    isstale boolean NOT NULL DEFAULT FALSE,
    labels jsonb NOT NULL,
    name text NOT NULL,
    observedat timestamp with time zone NOT NULL,
    scope text NOT NULL,
    servicenames jsonb NOT NULL,
    subnets jsonb NOT NULL,
    CONSTRAINT pk_swarmnetworkprojections PRIMARY KEY (platformid, dockernetworkid),
    CONSTRAINT fk_swarmnetworkprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE swarmnodeprojections (
    platformid uuid NOT NULL,
    dockernodeid text NOT NULL,
    address text NOT NULL,
    architecture text NOT NULL,
    availability text NOT NULL,
    desiredtaskcount integer NOT NULL,
    dockercreatedat timestamp with time zone,
    dockerupdatedat timestamp with time zone,
    engineversion text NOT NULL,
    hostname text NOT NULL,
    isleader boolean NOT NULL,
    isstale boolean NOT NULL DEFAULT FALSE,
    labels jsonb NOT NULL,
    observedat timestamp with time zone NOT NULL,
    operatingsystem text NOT NULL,
    reachability text NOT NULL,
    role text NOT NULL,
    runningtaskcount integer NOT NULL,
    status text NOT NULL,
    statusmessage text,
    versionindex bigint NOT NULL,
    CONSTRAINT pk_swarmnodeprojections PRIMARY KEY (platformid, dockernodeid),
    CONSTRAINT fk_swarmnodeprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE swarmsecretprojections (
    platformid uuid NOT NULL,
    dockersecretid text NOT NULL,
    dockercreatedat timestamp with time zone,
    dockerupdatedat timestamp with time zone,
    driver text,
    isstale boolean NOT NULL DEFAULT FALSE,
    labels jsonb NOT NULL,
    name text NOT NULL,
    observedat timestamp with time zone NOT NULL,
    servicenames jsonb NOT NULL,
    versionindex bigint NOT NULL,
    CONSTRAINT pk_swarmsecretprojections PRIMARY KEY (platformid, dockersecretid),
    CONSTRAINT fk_swarmsecretprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE swarmserviceprojections (
    platformid uuid NOT NULL,
    dockerserviceid text NOT NULL,
    configids jsonb NOT NULL,
    desiredtaskcount integer NOT NULL,
    dockercreatedat timestamp with time zone,
    dockerupdatedat timestamp with time zone,
    image text NOT NULL,
    isstale boolean NOT NULL DEFAULT FALSE,
    labels jsonb NOT NULL,
    mode text NOT NULL,
    name text NOT NULL,
    networkids jsonb NOT NULL,
    observedat timestamp with time zone NOT NULL,
    ports jsonb NOT NULL,
    runningtaskcount integer NOT NULL,
    secretids jsonb NOT NULL,
    updatemessage text,
    updatestate text NOT NULL,
    versionindex bigint NOT NULL,
    CONSTRAINT pk_swarmserviceprojections PRIMARY KEY (platformid, dockerserviceid),
    CONSTRAINT fk_swarmserviceprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE swarmtaskprojections (
    platformid uuid NOT NULL,
    dockertaskid text NOT NULL,
    desiredstate text NOT NULL,
    dockercreatedat timestamp with time zone,
    dockernodeid text NOT NULL,
    dockerserviceid text NOT NULL,
    dockerupdatedat timestamp with time zone,
    error text,
    image text NOT NULL,
    isstale boolean NOT NULL DEFAULT FALSE,
    name text NOT NULL,
    nodehostname text NOT NULL,
    observedat timestamp with time zone NOT NULL,
    ports jsonb NOT NULL,
    servicename text NOT NULL,
    slot integer,
    state text NOT NULL,
    statusmessage text,
    statustimestamp timestamp with time zone,
    versionindex bigint NOT NULL,
    CONSTRAINT pk_swarmtaskprojections PRIMARY KEY (platformid, dockertaskid),
    CONSTRAINT fk_swarmtaskprojections_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260803193803_migration0002', '10.0.10');

COMMIT;

