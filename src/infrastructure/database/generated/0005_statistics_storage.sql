-- Sample identity is already unique by source and capture time. These internal
-- UUIDs have no readers or foreign-key dependants; keep all observation values.
-- Reuse standalone unique indexes instead of building replacement history indexes.
ALTER TABLE containerstats DROP COLUMN id;
ALTER TABLE containerstats ADD CONSTRAINT pk_containerstats
    PRIMARY KEY USING INDEX ix_containerstats_containerid_created;
ALTER TABLE swarmservicestats DROP COLUMN id;
ALTER TABLE swarmservicestats ADD CONSTRAINT pk_swarmservicestats
    PRIMARY KEY USING INDEX ix_swarmservicestats_platformtaskcreated;

-- Constraint-owned unique indexes cannot be adopted as primary keys. Rebuild
-- only these two indexes in the same migration transaction, preserving uniqueness.
ALTER TABLE workloadstats DROP COLUMN id, DROP CONSTRAINT uq_workloadstats_container_created;
ALTER TABLE workloadstats ADD CONSTRAINT workloadstats_pkey PRIMARY KEY(containerid, created);
ALTER TABLE platformnodeiostats DROP COLUMN id,
    DROP CONSTRAINT platformnodeiostats_platformid_nodeid_created_key;
ALTER TABLE platformnodeiostats ADD CONSTRAINT platformnodeiostats_pkey
    PRIMARY KEY(platformid, nodeid, created);

-- Recycle expired tuples before the default 20% dead-row threshold. Keep the
-- server's vacuum cost limits and the existing seven-day retention policy.
ALTER TABLE containerstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE workloadstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE swarmservicestats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE platformstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE platformnodeiostats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE containerhealthtransitions SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
