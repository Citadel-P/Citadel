-- Page identities before probing history. Exact-source rechecks use the same bounds.
WITH page AS MATERIALIZED (
    SELECT id,platformid,nodeid,compacteduntil,coarseneduntil FROM platformhistorysources
    WHERE id>$4 AND ($3::bigint IS NULL OR id=$3)
    ORDER BY id LIMIT 64
)
SELECT src.id,src.platformid,first.created AS from_time, LEAST($1,(first.created/60+60)*60,last.created/60*60,pending.created/60*60) AS until
FROM page src
LEFT JOIN LATERAL (
    (SELECT created FROM platformstats WHERE src.nodeid='' AND platformid=src.platformid
        AND created>=GREATEST(src.compacteduntil,$2) AND created<$1 ORDER BY created LIMIT 1)
    UNION ALL
    (SELECT created FROM platformnodeiostats WHERE src.nodeid<>'' AND platformid=src.platformid AND nodeid=src.nodeid
        AND created>=GREATEST(src.compacteduntil,$2) AND created<$1 ORDER BY created LIMIT 1)
    ORDER BY created LIMIT 1
) first ON true
LEFT JOIN LATERAL (
    (SELECT created FROM platformstats WHERE first.created IS NOT NULL AND src.nodeid='' AND platformid=src.platformid ORDER BY created DESC LIMIT 1)
    UNION ALL
    (SELECT created FROM platformnodeiostats WHERE first.created IS NOT NULL AND src.nodeid<>'' AND platformid=src.platformid AND nodeid=src.nodeid ORDER BY created DESC LIMIT 1)
    ORDER BY created DESC LIMIT 1
) last ON first.created IS NOT NULL
LEFT JOIN LATERAL (
    SELECT created FROM platformstats WHERE first.created IS NOT NULL AND src.nodeid='' AND platformid=src.platformid AND alertpending
    ORDER BY created LIMIT 1
) pending ON true
ORDER BY src.id
