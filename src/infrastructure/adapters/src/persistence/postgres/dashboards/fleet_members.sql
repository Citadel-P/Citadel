, member_details AS NOT MATERIALIZED (
    SELECT o.kind, o.id, o.platform_id, o.name, o.state, p.name AS platform_name,
        CASE o.kind
            WHEN 'Container' THEN COALESCE(i.name, c.dockerimageid)
            WHEN 'Deployment' THEN COALESCE(d.spec->'Image'->>'ImageTag', d.spec->'Image'->>'ImageId')
            WHEN 'SwarmService' THEN s.spec->'Image'->>'ImageTag'
        END AS image,
        c.created AS container_created, COALESCE(d.createdat, r.createdat, s.createdat) AS workload_created,
        r.version AS release_version
    FROM owners o
    LEFT JOIN platforms p ON p.id = o.platform_id AND o.kind <> 'Platform'
        AND ($2 OR $8::integer IN (SELECT permission FROM global_grants)
             OR ($8, p.id) IN (SELECT permission, id FROM grants))
    LEFT JOIN containers c ON o.kind = 'Container' AND c.id = o.id
    LEFT JOIN images i ON i.dockerimageid = c.dockerimageid AND i.platformid = c.platformid
    LEFT JOIN deployments d ON o.kind = 'Deployment' AND d.id = o.id
    LEFT JOIN stacks st ON o.kind = 'Stack' AND st.id = o.id
    LEFT JOIN stackreleases r ON r.id = st.currentstackreleaseid
    LEFT JOIN swarmservices s ON o.kind = 'SwarmService' AND s.id = o.id
    WHERE $4::text IS NULL OR o.state = $4
), members AS NOT MATERIALIZED (
    SELECT * FROM member_details m
    WHERE $7::text IS NULL
        OR strpos(lower(m.name), lower($7)) > 0
        OR strpos(lower(m.platform_name), lower($7)) > 0
        OR strpos(lower(m.image), lower($7)) > 0
), page AS MATERIALIZED (
    SELECT * FROM members ORDER BY name, id LIMIT $5 OFFSET $6
), rows AS (
    SELECT p.kind, p.id, p.platform_id AS "platformId", p.name, p.state,
        p.platform_name AS "platformName", p.image, p.release_version AS "releaseVersion",
        CASE WHEN p.container_created > 0 THEN to_timestamp(p.container_created)
             ELSE p.workload_created END AS "createdAt",
        ports.values AS "publishedPorts", '[]'::jsonb AS reasons
    FROM page p
    LEFT JOIN containers c ON p.kind = 'Container' AND c.id = p.id
    LEFT JOIN LATERAL (
        SELECT jsonb_agg(port ORDER BY port) AS values
        FROM (
            -- Current binding maps and older Docker list projections both occur in stored data.
            SELECT DISTINCT host_port || '/' || protocol AS port
            FROM (
                SELECT COALESCE(binding->>'hostPort', binding->>'HostPort') AS host_port,
                    split_part(key, '/', 2) AS protocol
                FROM json_each(CASE WHEN json_typeof(c.ports) = 'object' THEN c.ports ELSE '{}'::json END)
                    CROSS JOIN LATERAL json_array_elements(
                        CASE WHEN json_typeof(value) = 'array' THEN value ELSE '[]'::json END
                    ) AS bound(binding)
                UNION ALL
                SELECT item->>'PublicPort', item->>'Type'
                FROM json_array_elements(CASE WHEN json_typeof(c.ports) = 'array' THEN c.ports ELSE '[]'::json END) AS legacy(item)
            ) bindings
            WHERE CASE WHEN host_port ~ '^[0-9]{1,5}$' THEN host_port::integer BETWEEN 1 AND 65535 ELSE false END
                AND protocol IN ('tcp', 'udp', 'sctp')
            ORDER BY port LIMIT 4
        ) published
    ) ports ON c.id IS NOT NULL
)
SELECT jsonb_build_object(
    'states', '[]'::jsonb, 'alerts', 0, 'problems', 0, 'updates', 0,
    'rows', COALESCE((SELECT jsonb_agg(to_jsonb(rows) ORDER BY name, id) FROM rows), '[]'),
    'total', (SELECT count(*) FROM members)
)
