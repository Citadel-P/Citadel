START TRANSACTION;
CREATE TABLE mfachallenges (
    id uuid NOT NULL,
    consumedat timestamp with time zone,
    createdat timestamp with time zone NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    failedattempts integer NOT NULL DEFAULT 0,
    userid uuid NOT NULL,
    CONSTRAINT pk_mfachallenges PRIMARY KEY (id),
    CONSTRAINT fk_mfachallenges_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE mfasetupsessions (
    id uuid NOT NULL,
    consumedat timestamp with time zone,
    createdat timestamp with time zone NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    protectedtotpsecret text NOT NULL,
    userid uuid NOT NULL,
    CONSTRAINT pk_mfasetupsessions PRIMARY KEY (id),
    CONSTRAINT fk_mfasetupsessions_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE usermfarecoverycodes (
    id uuid NOT NULL,
    codehash text NOT NULL,
    createdat timestamp with time zone NOT NULL,
    usedat timestamp with time zone,
    userid uuid NOT NULL,
    CONSTRAINT pk_usermfarecoverycodes PRIMARY KEY (id),
    CONSTRAINT fk_usermfarecoverycodes_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE TABLE usermfasettings (
    userid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    enabledat timestamp with time zone NOT NULL,
    lastacceptedtimestep bigint,
    protectedtotpsecret text NOT NULL,
    CONSTRAINT pk_usermfasettings PRIMARY KEY (userid),
    CONSTRAINT fk_usermfasettings_users_userid FOREIGN KEY (userid) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX ix_mfachallenges_expiresat ON mfachallenges (expiresat);

CREATE INDEX ix_mfachallenges_userid ON mfachallenges (userid);

CREATE INDEX ix_mfasetupsessions_expiresat ON mfasetupsessions (expiresat);

CREATE INDEX ix_mfasetupsessions_userid ON mfasetupsessions (userid);

CREATE INDEX ix_usermfarecoverycodes_userid ON usermfarecoverycodes (userid);

CREATE UNIQUE INDEX ix_usermfarecoverycodes_userid_codehash ON usermfarecoverycodes (userid, codehash);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260718112424_add_mfa', '10.0.10');

COMMIT;

