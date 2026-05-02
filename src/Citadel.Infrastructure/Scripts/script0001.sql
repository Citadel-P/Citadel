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
    action text NOT NULL,
    actorid uuid NOT NULL,
    resourceid uuid NOT NULL,
    resourcetype text NOT NULL,
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
    CONSTRAINT fk_activityevents_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE CASCADE
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
    resourceaction text NOT NULL,
    resourcetype text NOT NULL,
    roleid uuid NOT NULL,
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

INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('017b5a65-e312-8a67-b7fd-6124f6ddeeee', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0292fdeb-9a33-5a4c-60e9-395eac821cdc', 'Delete', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0309bcb2-05ec-623d-e45b-ec10cfddee24', 'Create', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0430e179-4cf1-193b-c54d-5d014203921b', 'Exec', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('052e45fb-cb15-9380-6a33-c233fde703a6', 'Update', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0854c122-21bc-147b-506b-6caf72ac48ca', 'View', 'Team', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0dd6ded1-5ef7-c1b5-a36a-d0de73459d8a', 'Apply', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0eda225f-cb5b-1bb0-9525-be92b14fc322', 'Apply', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0edd69d5-bb37-653c-9b25-5ff32a2b8243', 'Apply', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('0f030749-53d1-bade-8ef3-30112991786d', 'Create', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('11689210-d56f-4df8-9887-3338512e781d', 'Log', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('117176b6-ca23-e53d-d996-83affab7ed48', 'View', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('12bbcf07-7237-3afb-65f7-1fc8e2de4939', 'Apply', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('153c4670-ec3e-4037-6fd6-dd83bf29d3af', 'Log', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('15923c89-875e-7d0b-b80b-96af1f0cd1f2', 'Apply', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('17aefc62-767d-6e40-0a30-82d8cf360dec', 'Exec', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('18255099-e963-802b-21a3-d115440e9322', 'Pull', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('18b8b740-528c-6366-9902-ebf6a025d063', 'Apply', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('19357866-b0f7-4c0b-fb01-c3a6556d1e5f', 'Apply', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('1a0b6504-d508-0f6d-4513-12b80c3ab4d8', 'Update', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('1a84bf5e-dcf2-8a0b-8c4f-066f98ed2498', 'Exec', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('1f00afd4-a4d1-94cd-3a94-44d420eef066', 'Apply', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('21d8c7f7-5480-5509-e2ab-e3c8fdbb5ab8', 'Update', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('2548763c-c9b7-5359-a80a-5ce706c5c42c', 'Update', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('258c5870-adb0-f28f-bed2-5c993e5d11da', 'Pull', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('26bb8bc9-e526-dfd5-c3cc-2ebc0fb9837b', 'Apply', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('2941a68e-0eb8-2ba5-0d80-8ecf0dd21df8', 'Log', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('29b878e1-64b3-eee4-d8d9-7a7fb015c14c', 'Log', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('2ab50947-b044-4b44-6683-6fa76d3dd8e5', 'Exec', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('2e0582c8-9569-22cd-c777-69f03893b8ec', 'Delete', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('30f18293-2d41-2525-3210-e0f83bafa13d', 'Update', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('36667c46-31d9-3c62-1375-459c4daba3ed', 'Log', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('37023893-2218-6946-8caa-bbc8a7ad77a1', 'Log', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3a14f868-33fb-3a2e-92e1-5579da6962ce', 'Pull', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3a8c08b1-d033-1580-65f9-a1cb3ed3fc6a', 'Create', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3b12173b-62c2-740b-72bd-9727e893e58b', 'Exec', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3c380043-2b56-a61a-6855-8a8d1a50d9f1', 'Log', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3d84b4f0-2433-c34e-45e1-84d18b6c155d', 'Pull', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3e1abbe9-b2f2-21b8-bf02-38d4c10cd79d', 'Create', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('3e5365c6-7759-a0ad-e7fa-32263d00682c', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('40aadb71-124b-7c1b-44b9-f507a69ade11', 'Pull', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('41da5515-1e2d-bb3a-dd26-f13eb17fb81d', 'Exec', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('4711987b-af34-12f7-4cf4-795f51049571', 'Pull', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('47a25f07-9bb1-361d-788e-4d99fd0e50ee', 'Update', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('47c763f4-71e9-2992-3223-8ab97876b727', 'Apply', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('49ce8531-88f1-5ed2-a1c7-95d40cc72c47', 'Create', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('4aedeb0a-de43-b841-5970-9743bcde952b', 'Create', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('4b68a9cf-0af5-b0d6-8c05-e8b7e98b3919', 'View', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('4cfe0dcb-ce92-0500-981f-7d79b3782877', 'Create', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('4fa196f3-a9e5-7716-b1dd-aa574061e1f7', 'Pull', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('510688a3-f3e5-851a-29fb-8c8ae66a06d5', 'View', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('529f7f68-9ab0-a2f2-15af-a284b4d71ab6', 'Exec', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('55228106-7ae9-6748-5a33-73e253ad940d', 'Apply', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('57de5bf0-3ba6-3067-d4d0-c8c6a889507b', 'View', 'Role', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('58e018b1-ba4c-56bb-c56c-b9473688127b', 'Pull', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5b860b42-6a54-df7f-b375-2e1d1f47a563', 'Log', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5cd694be-92f2-0857-3c43-3fde801e6173', 'Log', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5e169a67-b789-1d24-db33-ea48a69f362e', 'Create', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5e8afc50-270c-4413-c5f1-bd25dac435d9', 'Apply', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5ede29a8-8e33-2d7c-48c3-1e5d733edd73', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('5ffa0070-42f0-4b37-efe5-4b0a394a1782', 'Log', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('613e9000-da2c-b4e7-9e02-b4bef349f0f7', 'Pull', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('62d97329-3b51-37c0-abe7-aba92734e97e', 'Pull', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('62e8e1fe-c911-e1b9-cc70-7efff6e08327', 'Pull', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('6395043d-510b-dc85-f19d-2a57463f4e8f', 'Pull', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('65540197-9169-5e94-9ad1-112ffe006a6d', 'Exec', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('65cb87df-133d-b577-21f6-2f20190ce45f', 'View', 'Registry', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('6deaa9b4-66e6-22bb-3f49-62584ccd9e1f', 'View', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('6e332b06-35e2-fbbd-e12c-bb7f02ab2474', 'Create', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('6ed1c4e3-9d28-23d8-2939-6a414aa0f53d', 'Apply', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('6f3a7126-e96a-d249-5e58-108bd0bd1f0f', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7238045c-c070-0ba5-b133-6083ea208d1b', 'Apply', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('723bb5cb-0c68-80e7-d890-7c4f6e3ce23a', 'Delete', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('73174250-b459-3def-d4e6-82d09d07ece9', 'Update', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('75575fd4-2d45-4302-12ba-1ebf9e9ea17f', 'Create', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('780d5066-5b19-668e-9f2f-0103f6cb23be', 'View', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('783ca30a-d153-8f1b-27db-c3e722f7f34d', 'Apply', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7862a71c-6493-24a1-84ca-dd2a630cc55a', 'Log', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7ad3c461-3f87-fe3c-a1e7-4a490906600e', 'Delete', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7ba78b50-a388-1606-e334-30c69758e60f', 'View', 'Deployment', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7ccb3b9e-a09a-d3c9-82ed-3f71646d2576', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7d64c2fa-810b-9866-5d31-be639383d279', 'Exec', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7defaa75-0f73-24b3-e32e-842bc1ec9885', 'Log', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('7fddc3c2-6d6e-cb92-a7a3-59a7a18b7c08', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('83ac5b37-8bd4-e093-3cdd-7e9f61300108', 'View', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8469f325-f132-73f4-0fa4-42131875a5ed', 'Update', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8487254d-0383-5b91-fa5f-816cfdc29054', 'View', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8658afec-8be0-2f4b-7b1e-478ac44341ec', 'Create', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8722b0da-7d07-7c14-0f9c-161e0c39a751', 'Pull', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('87d03608-e55e-6aed-7bb4-2a505eea1474', 'Exec', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('88a68bf5-8366-4d1a-f0fe-48225bc865bd', 'Exec', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('88dc9733-349d-635c-7ef6-829065f4f87b', 'Create', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8a0a4849-d9de-a63a-4e34-d25e431f6324', 'Log', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8b1633bc-d6ba-a419-38ea-e4d7e4b48cbe', 'Create', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8ce09606-e435-8a9b-2dab-8d1dfc91a198', 'Delete', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('8de4cd72-b2ce-4e18-f148-b93892845825', 'View', 'Alert', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('92694036-978b-d38d-81ed-8d28aeed9bd2', 'Log', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('92d404fe-a143-fba2-03cf-16479135fa81', 'Exec', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9308eb9b-7faa-e3d3-ffa8-86fc7946fae0', 'View', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9365df99-cab8-01da-f3c7-dc17a54e8801', 'Delete', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('961c1641-93aa-54ca-9b00-6608da3ae4c8', 'Pull', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('969a37a2-af1e-fa24-35b8-4857f802e001', 'Apply', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9982a665-f493-ce5f-9fd7-cd355f1ce257', 'Exec', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('99a0ba24-900d-448e-70f0-6e5eda10f8fc', 'Apply', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('99b23eff-a5c8-0cbe-6383-b99e43a43b23', 'Create', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9a7b1e84-3fd5-f750-2948-4a824aa66c82', 'Exec', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9aa863b1-84ad-e6c5-738f-41425290cbb8', 'Create', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9d3d40da-3f88-d22a-e596-adcb5e101a71', 'Log', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9e087c4f-e933-37c9-3941-b1803f83ede3', 'Create', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9f3d1be4-d19e-9453-86aa-2ae06eae6d18', 'Delete', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9fbad9ed-4795-61d8-e05b-748d2ee8aa30', 'Exec', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9fd296b8-cb43-5a62-9672-cf562aa6efd1', 'Update', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('9feb50a6-6f53-b270-5e7c-f679e7d85ed5', 'Update', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('a178007d-0c14-258e-bb6a-8828a5c28db7', 'Update', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('a1a791a5-9c37-88ad-c0e2-9a6717191298', 'Update', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('a7ac62e2-2a4d-50c6-6700-af7a5a345bf7', 'Update', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('a826cc2c-86ce-d61e-def2-e1ffa9bd5e89', 'Log', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('a9d333db-b006-8e96-192d-2c8444e2837d', 'Log', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ab32d40c-3859-6377-1634-a84b67e820dc', 'Apply', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ab3dfa51-423f-716d-a623-760c9f72f791', 'Pull', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('acab0152-cf67-d16c-fe1e-c579972ad2df', 'Pull', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('acece9ff-20d1-f3d5-c304-3a07adb9a03b', 'Update', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b19a2bbe-e59f-b092-4f40-20282533b1db', 'Exec', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b4101c57-b3f6-724d-8f9e-20b96ff470d3', 'Exec', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b503cc9e-5d58-8682-b509-5b5275d9ae5f', 'Exec', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b60ccb85-3aaa-0077-b0e8-7adbb9f4a596', 'View', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b678aa42-8c01-b706-1332-b85adb4e3096', 'Delete', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('b70ec2d0-198c-893d-9b83-f51cd8d4e5fd', 'Exec', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('bc2f22f1-0c5b-29f7-00e6-61f5fed77470', 'Log', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('bf01fa5c-a0a9-749e-b6c9-2d04af953b2b', 'Create', 'GitRepository', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c1184544-a092-3f80-c4d6-6f778db56b26', 'Update', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c12c9075-9343-73ec-322a-cc41a23230db', 'Delete', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c3e7230a-af2b-ba29-57ae-3c4043b66d61', 'View', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c4d01170-4f17-919d-9e91-210805644c7e', 'Log', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c71643e0-9549-edeb-c7ff-496efa58260c', 'Create', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c717528a-5a83-69a2-6893-4ab5fbc14d1b', 'View', 'Platform', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c75f0cb6-117e-8929-d133-c45f363c1610', 'Delete', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('c9ef58ef-d39e-eec5-7e23-dba79f2a1823', 'Log', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('cbcc1ffb-e622-9159-df1e-d0d05e50385c', 'Pull', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('cdd4e750-840f-2a08-7a9a-3bd65a4360e9', 'Delete', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ce566660-f041-32b6-0942-2f8abb92b17d', 'View', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('d23893f2-7f44-a593-83de-22ca4a8c80a1', 'Update', 'Registry', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('d41f9d7b-5371-c322-3049-cb166f19e83c', 'Exec', 'Stack', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('d5ba4edc-5278-a21e-613d-91350e52bce7', 'Apply', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('d868c269-b60f-2be0-2451-9b81b3c95674', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('d990b800-123d-a0ec-9b6a-239915235880', 'Create', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ddcb3cb1-e44f-0ab8-9e1e-698ed0352dc6', 'Apply', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('deb33289-b4e7-0111-9e93-079c08f09cf3', 'Create', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('df77eb4e-7860-5319-431e-481bfe08baeb', 'Pull', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e0494cd9-b3ec-b088-0532-089d029accca', 'Update', 'Team', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e08e5c0a-2e94-f112-22dd-e06d44dd2d9b', 'View', 'Stack', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e0e6d40a-91ae-d947-56a3-6e71df38581a', 'Update', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e13e141d-8322-f5f5-0488-cf961e919143', 'Update', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e3b44e1b-f776-b5ce-509d-2b529768a528', 'View', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e6a62042-68c0-d60f-2888-f685176a8f4f', 'Create', 'Deployment', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('e92ef59e-f1ab-51fc-abc8-4d90c98e5bf9', 'Update', 'Platform', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ea5f48d2-2719-0a78-dcb4-2efd447e674f', 'Pull', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('eaf65f71-81b1-b50f-041e-6f1471125bea', 'Exec', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ec427e29-a8ed-8604-59cc-7eda3268fc30', 'View', 'Alert', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ee63f887-67f1-a9f7-8028-31394316e680', 'Log', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('eef93be9-d327-6cfe-3bc9-5c5290f4b686', 'Apply', 'User', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ef3b1688-1b7b-f6c2-c4a2-169538d71970', 'Exec', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f12e902b-29c0-d404-41ef-6c7731211a70', 'Pull', 'Stack', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f1782e79-808e-d628-a83a-10b4639b9a68', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f30ff44f-86d5-8215-2c0f-60ed6ebd6234', 'Update', 'Registry', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f3e95d8a-2b63-3cb3-51ca-c5329b5b823e', 'Log', 'Deployment', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f43aa780-94d7-54b7-ceef-0cee3a2922df', 'View', 'Role', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f4595acc-f991-34d8-834c-4a1136010e17', 'View', 'Platform', '30000000-0000-0000-0000-000000000003');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f482aa00-8a5a-30da-4d1b-f0dfe770bcb3', 'Pull', 'User', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f7be80a4-dffa-1049-994a-5314ee03efaf', 'Create', 'GitAccount', '30000000-0000-0000-0000-000000000001');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f8833b18-d702-70b1-f75a-33f732e5ac29', 'Apply', 'Team', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f893a35b-7eac-bd31-921e-ec48bd5335e8', 'View', 'Role', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('f9af8f42-cf9c-21a8-43ba-793b4dd1bd3f', 'Pull', 'Alert', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('fc6bc1bc-bb69-098b-b09e-07a96444f57e', 'Update', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ff6e9bea-dbf2-e811-d4ba-6702a55c3f92', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000002');
INSERT INTO permissions (id, resourceaction, resourcetype, roleid)
VALUES ('ffc7419f-9c54-80fa-cac0-9e52ebeda6d3', 'View', 'User', '30000000-0000-0000-0000-000000000003');

INSERT INTO registries (id, configuration, createdat, createdbyactorid, description, name, registryhost, status)
VALUES ('00000000-0000-0000-0000-000000000100', '{
    "$type": "DockerHub"
}', TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', 'Public Docker Hub Registry', 'Docker Hub', 'hub.docker.com', 'Active');

INSERT INTO teams (id, actorid, name)
VALUES ('20000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000003', 'Default Team');

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

CREATE UNIQUE INDEX ix_platforms_address ON platforms (address);

CREATE UNIQUE INDEX ix_platformstats_platformid_created ON platformstats (platformid, created);

CREATE INDEX ix_refreshtokens_userid ON refreshtokens (userid);

CREATE INDEX ix_registries_createdbyactorid ON registries (createdbyactorid);

CREATE UNIQUE INDEX ix_registries_name ON registries (name);

CREATE INDEX ix_resourceaccesses_actor ON resourceaccesses (actorid);

CREATE INDEX ix_resourceaccesses_resourcetype_resourceid_actorid ON resourceaccesses (resourcetype, resourceid, actorid);

CREATE UNIQUE INDEX ix_resourceaccesses_resourcetype_resourceid_actorid_action ON resourceaccesses (resourcetype, resourceid, actorid, action);

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
VALUES ('20260502144828_migration0001', '10.0.7');

COMMIT;

