-- Seek through each source's timestamp index; never expand its JSON to find work.
SELECT src.id,src.platformid,first.created,
    LEAST($1,(first.created/60+60)*60,last.created/60*60,pending.created/60*60) AS until
FROM platformhistorysources src
CROSS JOIN LATERAL (
    (SELECT created FROM platformstats WHERE src.nodeid='' AND platformid=src.platformid
        AND created>=GREATEST(src.compacteduntil,$2) AND created<$1 ORDER BY created LIMIT 1)
    UNION ALL
    (SELECT created FROM platformnodeiostats WHERE src.nodeid<>'' AND platformid=src.platformid AND nodeid=src.nodeid
        AND created>=GREATEST(src.compacteduntil,$2) AND created<$1 ORDER BY created LIMIT 1)
    ORDER BY created LIMIT 1
) first
CROSS JOIN LATERAL (
    (SELECT created FROM platformstats WHERE src.nodeid='' AND platformid=src.platformid ORDER BY created DESC LIMIT 1)
    UNION ALL
    (SELECT created FROM platformnodeiostats WHERE src.nodeid<>'' AND platformid=src.platformid AND nodeid=src.nodeid ORDER BY created DESC LIMIT 1)
    ORDER BY created DESC LIMIT 1
) last
LEFT JOIN LATERAL (
    SELECT created FROM platformstats WHERE src.nodeid='' AND platformid=src.platformid AND alertpending
    ORDER BY created LIMIT 1
) pending ON true
WHERE ($3::bigint IS NULL OR src.id=$3)
    AND first.created<LEAST($1,(first.created/60+60)*60,last.created/60*60,pending.created/60*60)
ORDER BY first.created,src.id LIMIT 1
