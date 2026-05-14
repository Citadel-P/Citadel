CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    "MigrationId" character varying(150) NOT NULL,
    "ProductVersion" character varying(32) NOT NULL,
    CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY ("MigrationId")
);

START TRANSACTION;
CREATE TABLE actors (
    id uuid NOT NULL,
    isenabled boolean NOT NULL DEFAULT TRUE,
    type text NOT NULL,
    CONSTRAINT pk_actors PRIMARY KEY (id)
);

CREATE TABLE platforms (
    id uuid NOT NULL,
    address text NOT NULL,
    agentversion text,
    connectortype text NOT NULL,
    cpucount integer NOT NULL,
    imagecount integer NOT NULL,
    memtotal bigint NOT NULL,
    name text NOT NULL,
    networkcount integer NOT NULL,
    platformdescriptor json NOT NULL,
    serverversion text,
    status text NOT NULL,
    volumecount integer NOT NULL,
    CONSTRAINT pk_platforms PRIMARY KEY (id)
);

CREATE TABLE resourceaccesses (
    id uuid NOT NULL,
    actorid uuid NOT NULL,
    permissionlevel integer NOT NULL,
    resourceid uuid NOT NULL,
    resourcetype integer NOT NULL,
    specificpermissions integer NOT NULL,
    CONSTRAINT pk_resourceaccesses PRIMARY KEY (id)
);

CREATE TABLE roles (
    id uuid NOT NULL,
    name text NOT NULL,
    roletype text NOT NULL,
    CONSTRAINT pk_roles PRIMARY KEY (id)
);

CREATE TABLE alertchannels (
    id uuid NOT NULL,
    alertdestination text NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    isactive boolean NOT NULL DEFAULT TRUE,
    name text NOT NULL,
    url text NOT NULL,
    CONSTRAINT pk_alertchannels PRIMARY KEY (id),
    CONSTRAINT fk_alertchannels_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE alertrules (
    id uuid NOT NULL,
    cooldownseconds integer,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    description text,
    limitedto json NOT NULL,
    name text NOT NULL,
    quiethours json NOT NULL,
    requiredmatches integer,
    severity text NOT NULL,
    status text NOT NULL DEFAULT 'Enabled',
    threshold double precision,
    type text NOT NULL,
    CONSTRAINT pk_alertrules PRIMARY KEY (id),
    CONSTRAINT fk_alertrules_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE gitaccounts (
    id uuid NOT NULL,
    authtype text NOT NULL,
    configuration json NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    domain text NOT NULL,
    name text NOT NULL,
    transport text NOT NULL,
    CONSTRAINT pk_gitaccounts PRIMARY KEY (id),
    CONSTRAINT fk_gitaccounts_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE registries (
    id uuid NOT NULL,
    configuration json NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    description text,
    name text NOT NULL,
    registryhost text NOT NULL,
    status text NOT NULL,
    CONSTRAINT pk_registries PRIMARY KEY (id),
    CONSTRAINT fk_registries_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE stacks (
    id uuid NOT NULL,
    controlstartedat bigint,
    controlstate text DEFAULT 'Idle',
    controltriggeredby uuid,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    currentstackreleaseid uuid,
    description text,
    name text NOT NULL,
    rowversion bigint NOT NULL DEFAULT 0,
    stacksource text NOT NULL,
    stackupdatestate json NOT NULL,
    CONSTRAINT pk_stacks PRIMARY KEY (id),
    CONSTRAINT fk_stacks_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_stacks_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE teams (
    id uuid NOT NULL,
    actorid uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT pk_teams PRIMARY KEY (id),
    CONSTRAINT fk_teams_actors_actorid FOREIGN KEY (actorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE users (
    id uuid NOT NULL,
    actorid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    email text,
    name text NOT NULL,
    password text,
    CONSTRAINT pk_users PRIMARY KEY (id),
    CONSTRAINT fk_users_actors_actorid FOREIGN KEY (actorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_users_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE activityevents (
    id uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    eventtype text NOT NULL,
    info text NOT NULL,
    platformid uuid,
    resourceid uuid,
    resourcename text NOT NULL,
    resourcetype text NOT NULL,
    status text NOT NULL,
    CONSTRAINT pk_activityevents PRIMARY KEY (id),
    CONSTRAINT fk_activityevents_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_activityevents_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE SET NULL
);

CREATE TABLE deployments (
    id uuid NOT NULL,
    autoupdatestate_currentdigest text,
    autoupdatestate_lastcheckedat timestamp with time zone,
    autoupdatestate_lasterror text,
    autoupdatestate_remotedigest text,
    autoupdatestate_status text,
    controlstartedat bigint,
    controlstate text DEFAULT 'Idle',
    controltriggeredby uuid,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    description text,
    name text NOT NULL,
    platformid uuid NOT NULL,
    rowversion bigint NOT NULL DEFAULT 0,
    spec json NOT NULL,
    status text NOT NULL,
    CONSTRAINT pk_deployments PRIMARY KEY (id),
    CONSTRAINT fk_deployments_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_deployments_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_deployments_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT
);

CREATE TABLE platformstats (
    id uuid NOT NULL,
    cpuusage double precision NOT NULL,
    created bigint NOT NULL,
    memoryusage double precision NOT NULL,
    platformid uuid NOT NULL,
    rxbytes double precision NOT NULL,
    txbytes double precision NOT NULL,
    CONSTRAINT pk_platformstats PRIMARY KEY (id),
    CONSTRAINT fk_platformstats_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE actorroles (
    actorid uuid NOT NULL,
    roleid uuid NOT NULL,
    CONSTRAINT pk_actorroles PRIMARY KEY (actorid, roleid),
    CONSTRAINT fk_actorroles_actors_actorid FOREIGN KEY (actorid) REFERENCES actors (id) ON DELETE CASCADE,
    CONSTRAINT fk_actorroles_roles_roleid FOREIGN KEY (roleid) REFERENCES roles (id) ON DELETE CASCADE
);

CREATE TABLE permissions (
    id uuid NOT NULL,
    permissionlevel integer NOT NULL,
    resourcetype integer NOT NULL,
    roleid uuid NOT NULL,
    specificpermissions integer NOT NULL,
    CONSTRAINT pk_permissions PRIMARY KEY (id),
    CONSTRAINT fk_permissions_roles_roleid FOREIGN KEY (roleid) REFERENCES roles (id) ON DELETE CASCADE
);

CREATE TABLE alertevents (
    id uuid NOT NULL,
    acknowledgedat timestamp with time zone,
    acknowledgedbyactorid uuid,
    alertruleid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    deduplicationkey text NOT NULL,
    info json NOT NULL,
    openincidentkey text,
    resolutionnote text,
    resolvedat timestamp with time zone,
    resolvedbyactorid uuid,
    resourceid uuid,
    resourcename text NOT NULL,
    resourcetype text NOT NULL,
    severity text NOT NULL,
    type text NOT NULL,
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_alertevents PRIMARY KEY (id),
    CONSTRAINT fk_alertevents_alertrules_alertruleid FOREIGN KEY (alertruleid) REFERENCES alertrules (id) ON DELETE CASCADE
);

CREATE TABLE alertrulechannels (
    alertruleid uuid NOT NULL,
    alertchannelid uuid NOT NULL,
    CONSTRAINT pk_alertrulechannels PRIMARY KEY (alertruleid, alertchannelid),
    CONSTRAINT fk_alertrulechannels_alertchannels_alertchannelid FOREIGN KEY (alertchannelid) REFERENCES alertchannels (id) ON DELETE CASCADE,
    CONSTRAINT fk_alertrulechannels_alertrules_alertruleid FOREIGN KEY (alertruleid) REFERENCES alertrules (id) ON DELETE CASCADE
);

CREATE TABLE alertrulestates (
    alertruleid uuid NOT NULL,
    resourceid uuid NOT NULL,
    consecutivematches integer NOT NULL DEFAULT 3,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    lasttriggeredat timestamp with time zone,
    CONSTRAINT pk_alertrulestates PRIMARY KEY (alertruleid, resourceid),
    CONSTRAINT fk_alertrulestates_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_alertrulestates_alertrules_alertruleid FOREIGN KEY (alertruleid) REFERENCES alertrules (id) ON DELETE CASCADE
);

CREATE TABLE gitrepositories (
    id uuid NOT NULL,
    controlstartedat bigint,
    controlstate text DEFAULT 'Idle',
    controltriggeredby uuid,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    defaultbranch text NOT NULL,
    description text,
    gitaccountid uuid,
    name text NOT NULL,
    onclone text,
    onpull text,
    rowversion bigint NOT NULL DEFAULT 0,
    status text NOT NULL,
    url text NOT NULL,
    webhookenabled boolean NOT NULL DEFAULT FALSE,
    webhooksecret text,
    CONSTRAINT pk_gitrepositories PRIMARY KEY (id),
    CONSTRAINT fk_gitrepositories_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_gitrepositories_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_gitrepositories_gitaccounts_gitaccountid FOREIGN KEY (gitaccountid) REFERENCES gitaccounts (id) ON DELETE SET NULL
);

CREATE TABLE images (
    id uuid NOT NULL,
    containers integer NOT NULL DEFAULT 0,
    controlstartedat bigint,
    controlstate text DEFAULT 'Idle',
    controltriggeredby uuid,
    createdat timestamp with time zone NOT NULL,
    dockerimageid text NOT NULL,
    name text NOT NULL,
    platformid uuid NOT NULL,
    registryid uuid,
    rowversion bigint NOT NULL DEFAULT 0,
    size double precision NOT NULL DEFAULT 0.0,
    tags json NOT NULL,
    updatedat timestamp with time zone,
    CONSTRAINT pk_images PRIMARY KEY (id),
    CONSTRAINT fk_images_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_images_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE,
    CONSTRAINT fk_images_registries_registryid FOREIGN KEY (registryid) REFERENCES registries (id) ON DELETE SET NULL
);

CREATE TABLE stackreleases (
    id uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    platformid uuid NOT NULL,
    spec json NOT NULL,
    stackid uuid NOT NULL,
    status text NOT NULL,
    version text NOT NULL,
    CONSTRAINT pk_stackreleases PRIMARY KEY (id),
    CONSTRAINT fk_stackreleases_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_stackreleases_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT,
    CONSTRAINT fk_stackreleases_stacks_stackid FOREIGN KEY (stackid) REFERENCES stacks (id) ON DELETE CASCADE
);

CREATE TABLE refreshtokens (
    id uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    userid uuid NOT NULL,
    CONSTRAINT pk_refreshtokens PRIMARY KEY (id),
    CONSTRAINT fk_refreshtokens_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE usersteams (
    userid uuid NOT NULL,
    teamid uuid NOT NULL,
    CONSTRAINT pk_usersteams PRIMARY KEY (userid, teamid),
    CONSTRAINT fk_usersteams_teams_teamid FOREIGN KEY (teamid) REFERENCES teams (id) ON DELETE CASCADE,
    CONSTRAINT fk_usersteams_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE containers (
    id uuid NOT NULL,
    controlstartedat bigint,
    controlstate text DEFAULT 'Idle',
    controltriggeredby uuid,
    created bigint NOT NULL,
    deploymentid uuid,
    dockercontainerid text NOT NULL,
    dockerimageid text NOT NULL,
    imageid uuid,
    name text NOT NULL,
    platformid uuid NOT NULL,
    ports json NOT NULL,
    rowversion bigint NOT NULL DEFAULT 0,
    stack text,
    state text NOT NULL,
    updated bigint NOT NULL,
    CONSTRAINT pk_containers PRIMARY KEY (id),
    CONSTRAINT fk_containers_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_containers_deployments_deploymentid FOREIGN KEY (deploymentid) REFERENCES deployments (id) ON DELETE SET NULL,
    CONSTRAINT fk_containers_images_imageid FOREIGN KEY (imageid) REFERENCES images (id) ON DELETE SET NULL,
    CONSTRAINT fk_containers_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
);

CREATE TABLE containerstats (
    id uuid NOT NULL,
    containerid uuid NOT NULL,
    cpuusage double precision NOT NULL,
    created bigint NOT NULL,
    memoryactive double precision NOT NULL,
    memorycache double precision NOT NULL,
    memorylimit double precision NOT NULL,
    rxbytes double precision NOT NULL,
    txbytes double precision NOT NULL,
    CONSTRAINT pk_containerstats PRIMARY KEY (id),
    CONSTRAINT fk_containerstats_containers_containerid FOREIGN KEY (containerid) REFERENCES containers (id) ON DELETE CASCADE
);

INSERT INTO actors (id, isenabled, type)
VALUES ('00000000-0000-0000-0000-000000000001', TRUE, 'System');
INSERT INTO actors (id, isenabled, type)
VALUES ('00000000-0000-0000-0000-000000000002', TRUE, 'User');
INSERT INTO actors (id, isenabled, type)
VALUES ('00000000-0000-0000-0000-000000000003', TRUE, 'Team');

INSERT INTO roles (id, name, roletype)
VALUES ('30000000-0000-0000-0000-000000000001', 'Admin', 'System');
INSERT INTO roles (id, name, roletype)
VALUES ('30000000-0000-0000-0000-000000000002', 'Operator', 'System');
INSERT INTO roles (id, name, roletype)
VALUES ('30000000-0000-0000-0000-000000000003', 'Viewer', 'System');

INSERT INTO actorroles (actorid, roleid)
VALUES ('00000000-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000001');
INSERT INTO actorroles (actorid, roleid)
VALUES ('00000000-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000002');

INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000001', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'CPU > 90% - Platform', '[]', 3, 'Critical', 90.0, 'PlatformCpuHigh');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000002', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'RAM > 90% - Platform', '[]', 3, 'Critical', 90.0, 'PlatformRamHigh');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000003', 600, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Platform Unreachable', '[]', NULL, 'Critical', NULL, 'PlatformUnreachable');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000004', 3600, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Platform Version Mismatch', '[]', NULL, 'Warning', NULL, 'PlatformVersionMismatch');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000005', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Unmanaged Container Created', '[]', NULL, 'Info', NULL, 'UnmanagedContainerCreated');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000006', 86400, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Image Update Available - Deployment', '[]', NULL, 'Info', NULL, 'DeploymentImageUpdateAvailable');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000007', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Auto Deploy Failed - Deployment', '[]', NULL, 'Critical', NULL, 'DeploymentAutoDeployFailed');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000008', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Deployment Auto Updated', '[]', NULL, 'Info', NULL, 'DeploymentAutoUpdated');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000009', 86400, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Image Update Available - Stack', '[]', NULL, 'Info', NULL, 'StackImageUpdateAvailable');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-00000000000a', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Auto Deploy Failed - Stack', '[]', NULL, 'Critical', NULL, 'StackAutoDeployFailed');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-00000000000b', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Stack Auto Updated', '[]', NULL, 'Info', NULL, 'StackAutoUpdated');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000011', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'CPU > 80% - Platform', '[]', 3, 'Warning', 80.0, 'PlatformCpuHigh');
INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000022', 300, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'RAM > 80% - Platform', '[]', 3, 'Warning', 80.0, 'PlatformRamHigh');

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('030c8f34-4447-d6b0-bc28-62b9626999c7', 0, 1, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('07b143ad-6b02-c7ff-3d7d-48af137b2bbc', 1, 0, '30000000-0000-0000-0000-000000000002', 27);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('1e6772eb-d9b9-ef19-565e-14ae0f6691d0', 1, 2, '30000000-0000-0000-0000-000000000002', 15);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('2533e6f2-53e3-1281-01f7-cc28045cbc4f', 1, 10, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('2d9c5d81-bce2-e0a6-004b-138d3ac0a4a9', 2, 3, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('361e1bf8-ef0f-1409-9137-6fa885696a19', 2, 7, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('3bf8e221-07a0-052c-d831-2c82d22f7660', 1, 9, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('4032d1e2-fe5e-16ef-f554-69bd2c2ac19d', 0, 10, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('440deb9d-ace8-5e15-ef80-3a42f11a0c42', 2, 10, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('44debca1-5d97-b691-196c-8e421143e307', 1, 8, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('645b4c54-7937-2180-7186-be24ac6bf330', 2, 5, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('672ebf04-40e5-547b-29f2-6daf5c3c3856', 0, 8, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('68b977f5-a0d2-7235-a7ff-1b68baeb943f', 1, 1, '30000000-0000-0000-0000-000000000002', 31);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('76e4e5dc-c649-c02d-e754-e41cc49955d5', 2, 1, '30000000-0000-0000-0000-000000000001', 31);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('80aa1c34-79dd-6587-52db-52605326fe77', 1, 7, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('86dadd60-fced-3dcd-cdbe-8d262bec7d22', 1, 5, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('909763b4-50a0-e1c7-6df1-61add076910c', 0, 2, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('936632a5-4e74-0a17-fb8e-497c960c3005', 0, 0, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('94717f37-cc1a-de60-9bca-dc6379444bfb', 0, 5, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('97597a3e-c415-667b-039a-a7a287daefea', 0, 4, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('987e89d0-2c8f-87d8-830f-7461a7db392e', 2, 4, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('9b075e03-6326-7b95-ae78-2b296990ce26', 1, 4, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('a4b222e3-7452-b5f4-377b-c05652533f67', 2, 0, '30000000-0000-0000-0000-000000000001', 27);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('a60ba8de-ff46-b387-85ed-913d96170a2a', 0, 7, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('b104e60e-87ef-a58f-f04a-ba2fc634f037', 0, 6, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('b3abb382-80da-8170-b011-05af044e7908', 1, 3, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('bc9bb12f-b72b-a750-3ba3-49a8b6574858', 2, 2, '30000000-0000-0000-0000-000000000001', 15);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('d5fa8563-b0a2-4f11-7e16-7c1877e43dda', 2, 6, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('dbb104e4-d7e2-5173-b0b2-6d1519c2f682', 2, 8, '30000000-0000-0000-0000-000000000001', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('e04cd0d3-47bf-2d28-e099-c7a9b61e3875', 0, 3, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('ee9254c3-9b59-15a0-aa85-898f5974a603', 1, 6, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('fb710f21-c146-e381-00f0-820f58ecb69a', 0, 9, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('fbb8ef70-2ec3-134f-0c18-1533173d5849', 2, 9, '30000000-0000-0000-0000-000000000001', 0);

INSERT INTO registries (id, configuration, createdat, createdbyactorid, description, name, registryhost, status)
VALUES ('00000000-0000-0000-0000-000000000100', '{
    "$type": "DockerHub"
}', TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', 'Public Docker Hub Registry', 'Docker Hub', 'hub.docker.com', 'Active');

INSERT INTO teams (id, actorid, name)
VALUES ('20000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000003', 'Operators');

INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password)
VALUES ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002', TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', 'admin@citadel.local', 'admin', 'o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1');

CREATE INDEX ix_activityevents_createdbyactorid ON activityevents (createdbyactorid);

CREATE INDEX ix_activityevents_eventtype ON activityevents (eventtype);

CREATE INDEX ix_activityevents_platform_createdat ON activityevents (platformid, createdat);

CREATE INDEX ix_activityevents_resource_createdat ON activityevents (resourceid, createdat);

CREATE INDEX ix_activityevents_status ON activityevents (status);

CREATE INDEX ix_actorroles_actorid ON actorroles (actorid);

CREATE INDEX ix_actorroles_roleid ON actorroles (roleid);

CREATE INDEX ix_alertchannels_createdbyactorid ON alertchannels (createdbyactorid);

CREATE INDEX ix_alertevents_alertruleid ON alertevents (alertruleid);

CREATE UNIQUE INDEX ix_alertevents_openincidentkey ON alertevents (openincidentkey);

CREATE INDEX ix_alertevents_resource_createdat ON alertevents (resourceid, createdat);

CREATE INDEX ix_alertevents_resourcetype ON alertevents (resourcetype);

CREATE INDEX ix_alertevents_type ON alertevents (type);

CREATE INDEX ix_alertrulechannels_alertchannelid ON alertrulechannels (alertchannelid);

CREATE INDEX ix_alertrules_createdbyactorid ON alertrules (createdbyactorid);

CREATE INDEX ix_alertrules_type ON alertrules (type);

CREATE INDEX ix_alertrulestates_createdbyactorid ON alertrulestates (createdbyactorid);

CREATE UNIQUE INDEX ix__containers_dockercontainerid_platformid ON containers (dockercontainerid, platformid);

CREATE INDEX ix_containers_controltriggeredby ON containers (controltriggeredby);

CREATE INDEX ix_containers_deploymentid ON containers (deploymentid);

CREATE INDEX ix_containers_dockerimageid ON containers (dockerimageid);

CREATE INDEX ix_containers_imageid ON containers (imageid);

CREATE INDEX ix_containers_platformid ON containers (platformid);

CREATE UNIQUE INDEX ix_containerstats_containerid_created ON containerstats (containerid, created);

CREATE INDEX ix_deployments_controltriggeredby ON deployments (controltriggeredby);

CREATE INDEX ix_deployments_createdbyactorid ON deployments (createdbyactorid);

CREATE UNIQUE INDEX ix_deployments_name_platformid ON deployments (name, platformid);

CREATE INDEX ix_deployments_platformid ON deployments (platformid);

CREATE INDEX ix_gitaccounts_createdbyactorid ON gitaccounts (createdbyactorid);

CREATE UNIQUE INDEX ix_gitaccounts_name ON gitaccounts (name);

CREATE INDEX ix_gitrepositories_controltriggeredby ON gitrepositories (controltriggeredby);

CREATE INDEX ix_gitrepositories_createdbyactorid ON gitrepositories (createdbyactorid);

CREATE INDEX ix_gitrepositories_gitaccountid ON gitrepositories (gitaccountid);

CREATE UNIQUE INDEX ix_gitrepositories_name ON gitrepositories (name);

CREATE INDEX ix_images_controltriggeredby ON images (controltriggeredby);

CREATE UNIQUE INDEX ix_images_dockerimageid_platformid ON images (dockerimageid, platformid);

CREATE INDEX ix_images_platformid ON images (platformid);

CREATE INDEX ix_images_registryid ON images (registryid);

CREATE INDEX ix_permissions_roleid ON permissions (roleid);

CREATE UNIQUE INDEX ix_permissions_roleid_resourcetype ON permissions (roleid, resourcetype);

CREATE UNIQUE INDEX ix_platforms_address ON platforms (address);

CREATE UNIQUE INDEX ix_platformstats_platformid_created ON platformstats (platformid, created);

CREATE INDEX ix_refreshtokens_userid ON refreshtokens (userid);

CREATE INDEX ix_registries_createdbyactorid ON registries (createdbyactorid);

CREATE UNIQUE INDEX ix_registries_name ON registries (name);

CREATE INDEX ix_resourceaccesses_actor ON resourceaccesses (actorid);

CREATE UNIQUE INDEX ix_resourceaccesses_resourcetype_resourceid_actorid ON resourceaccesses (resourcetype, resourceid, actorid);

CREATE INDEX ix_stackreleases_createdbyactorid ON stackreleases (createdbyactorid);

CREATE INDEX ix_stackreleases_platformid ON stackreleases (platformid);

CREATE INDEX ix_stackreleases_stackid ON stackreleases (stackid);

CREATE INDEX ix_stacks_controltriggeredby ON stacks (controltriggeredby);

CREATE INDEX ix_stacks_createdbyactorid ON stacks (createdbyactorid);

CREATE INDEX ix_stacks_currentstackreleaseid ON stacks (currentstackreleaseid);

CREATE UNIQUE INDEX ix_teams_actorid ON teams (actorid);

CREATE UNIQUE INDEX ix_users_actorid ON users (actorid);

CREATE INDEX ix_users_createdbyactorid ON users (createdbyactorid);

CREATE UNIQUE INDEX ix_users_email ON users (email);

CREATE INDEX ix_usersteams_teamid ON usersteams (teamid);

CREATE INDEX ix_usersteams_userid ON usersteams (userid);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260514141339_migration0001', '10.0.7');

COMMIT;

