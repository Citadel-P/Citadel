CREATE TYPE workload_state AS ENUM ('Pending', 'Running', 'Failed');

CREATE TABLE owners (
    id uuid NOT NULL,
    name text NOT NULL,
    CONSTRAINT pk_owners PRIMARY KEY (id),
    CONSTRAINT uq_owners_name UNIQUE (name)
);

CREATE TABLE workloads (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    -- atlas:renamed_from external_id
    runtime_id text NOT NULL,
    state workload_state NOT NULL DEFAULT 'Pending',
    spec jsonb NOT NULL DEFAULT '{}'::jsonb,
    replicas integer NOT NULL DEFAULT 1,
    deleted_at timestamp with time zone,
    CONSTRAINT pk_workloads PRIMARY KEY (id),
    CONSTRAINT fk_workloads_owner FOREIGN KEY (owner_id) REFERENCES owners (id) ON DELETE CASCADE,
    CONSTRAINT ck_workloads_replicas CHECK (replicas >= 0)
);

-- atlas:renamed_from ux_workloads_external_id
CREATE UNIQUE INDEX ux_workloads_runtime_id ON workloads (runtime_id);
CREATE UNIQUE INDEX ux_workloads_active_owner_external_id
    ON workloads (owner_id, runtime_id)
    WHERE deleted_at IS NULL;
