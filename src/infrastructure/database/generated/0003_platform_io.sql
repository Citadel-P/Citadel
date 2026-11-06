-- Retain physical container I/O counters with platform telemetry, independently of inventory.
ALTER TABLE platformstats ADD COLUMN containerio jsonb;

-- Node I/O outlives inventory and task projections, like platform host samples.
CREATE TABLE platformnodeiostats (
    id uuid PRIMARY KEY,
    platformid uuid NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    nodeid text NOT NULL,
    created bigint NOT NULL,
    containerio jsonb NOT NULL,
    UNIQUE(platformid, nodeid, created)
);
CREATE INDEX ix_platformnodeiostats_platform_created ON platformnodeiostats(platformid, created);
CREATE INDEX ix_platformnodeiostats_created ON platformnodeiostats(created);
