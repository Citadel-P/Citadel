SELECT json_build_object(
    'instance', (SELECT instanceid FROM public.citadelinstanceidentity WHERE id = 1),
    'initialized', EXISTS(SELECT 1 FROM public.instancesetupstates WHERE id = 1 AND initializedat IS NOT NULL),
    'migrations', (SELECT json_agg(m) FROM (
        SELECT id, checksum, completed_at IS NOT NULL AS completed, failure IS NOT NULL AS failed
        FROM public.citadel_schema_migrations ORDER BY id LIMIT 1025
    ) m),
    'owners', (SELECT json_agg(o) FROM (
        SELECT id, instanceid AS instance, actorid AS actor, idempotencykey AS key, revision, operation
        FROM public.instanceupdates WHERE ownsinstallation ORDER BY id LIMIT 2
    ) o)
);
