CREATE TABLE dashboards (
    id uuid PRIMARY KEY,
    owneractorid uuid REFERENCES actors(id) ON DELETE SET NULL,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 128),
    visibility text NOT NULL CHECK (visibility IN ('Private','InstanceShared','Restricted')),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    configuration jsonb NOT NULL CHECK (jsonb_typeof(configuration) = 'object'),
    createdat timestamptz NOT NULL DEFAULT now(),
    updatedat timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_dashboards_picker ON dashboards(lower(name), id);
CREATE INDEX ix_dashboards_private_owner ON dashboards(owneractorid) WHERE visibility='Private';

CREATE TABLE personaldashboarddefaults (
    actorid uuid PRIMARY KEY REFERENCES actors(id) ON DELETE CASCADE,
    dashboardid uuid NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE
);

CREATE TABLE instancedashboarddefault (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    dashboardid uuid NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE
);
