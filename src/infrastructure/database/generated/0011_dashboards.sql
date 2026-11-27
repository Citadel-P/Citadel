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
CREATE UNIQUE INDEX ix_dashboards_unique_name ON dashboards (lower(name));

CREATE TABLE personaldashboarddefaults (
    actorid uuid PRIMARY KEY REFERENCES actors(id) ON DELETE CASCADE,
    dashboardid uuid NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE
);

CREATE TABLE instancedashboarddefault (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    dashboardid uuid NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE
);

ALTER TABLE userpreferences ALTER COLUMN themecolor SET DEFAULT 'Blue';

CREATE FUNCTION notify_dashboard_activity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('citadel_dashboard_changes', 'activity');
    RETURN NULL;
END;
$$;

CREATE TRIGGER dashboard_activity_changed AFTER INSERT OR UPDATE OR DELETE ON activityevents
    FOR EACH STATEMENT EXECUTE FUNCTION notify_dashboard_activity();

CREATE FUNCTION notify_dashboard_definition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_notify('citadel_dashboard_changes', COALESCE(NEW.id, OLD.id)::text);
    RETURN NULL;
END;
$$;

CREATE TRIGGER dashboard_definition_changed AFTER INSERT OR UPDATE OR DELETE ON dashboards
    FOR EACH ROW EXECUTE FUNCTION notify_dashboard_definition();

-- Create the initial home dashboard.
WITH example AS (
    INSERT INTO dashboards (id, name, visibility, configuration)
    VALUES (gen_random_uuid(), 'Overview', 'InstanceShared', '{
        "schemaVersion": 2,
        "rangeMinutes": 60,
        "tags": [],
        "widgets": [
            {
                "id": "a3072200-ea4f-4dc9-b68a-befac4aad24b",
                "title": "Fleet summary",
                "layout": {"x": 0, "y": 0, "w": 12, "h": 2},
                "settings": {"kind": "FleetSummary", "measures": ["Platforms", "Workloads", "Alerts"]}
            },
            {
                "id": "f714c882-29dc-4e4c-9b65-af4ed73aef7b",
                "title": "Needs attention",
                "layout": {"x": 0, "y": 2, "w": 7, "h": 5},
                "settings": {"kind": "NeedsAttention", "kinds": [], "limit": 5, "tab": "All"}
            },
            {
                "id": "931c66a9-89dd-4174-824d-ad80d87be00c",
                "title": "Deployment state",
                "layout": {"x": 7, "y": 2, "w": 5, "h": 5},
                "settings": {"kind": "ResourceState", "resourceKind": "Deployment"}
            },
            {
                "id": "35f95c9c-fefd-4a3b-9b0d-a6239aee0214",
                "title": "Recent activity",
                "layout": {"x": 0, "y": 7, "w": 12, "h": 5},
                "settings": {"kind": "RecentActivity", "limit": 5}
            }
        ]
    }'::jsonb)
    RETURNING id
)
INSERT INTO instancedashboarddefault (dashboardid)
SELECT id FROM example;

CREATE INDEX ix_backupruns_policy_completed ON backupruns (backuppolicyid, completedat DESC, id DESC)
    INCLUDE (status) WHERE completedat IS NOT NULL AND status <> 'Cancelled';

CREATE INDEX ix_alertevents_unresolved_resource ON alertevents (resourceid)
    WHERE resolvedat IS NULL;

CREATE INDEX ix_swarmtasks_running_service ON swarmtaskprojections (platformid, dockerserviceid)
    WHERE NOT isstale AND lower(state) = 'running' AND lower(desiredstate) = 'running';
