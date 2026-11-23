-- Page identities before probing history. Exact-source rechecks use the same bounds.
WITH page AS MATERIALIZED (
    SELECT id,platformid,nodeid,compacteduntil,coarseneduntil FROM platformhistorysources
    WHERE id>$3 AND ($2::bigint IS NULL OR id=$2)
    ORDER BY id LIMIT 64
)
SELECT src.id,src.platformid,first.created/900*900 AS from_time,first.created/900*900+900 AS until
FROM page src
LEFT JOIN LATERAL (
    SELECT created FROM platformiorollups
    WHERE sourceid>=src.id AND sourceid<=src.id
        AND created>=GREATEST(src.coarseneduntil,($1+899)/900*900)
        AND created<src.compacteduntil/900*900
    ORDER BY sourceid,created LIMIT 1
) first ON true
ORDER BY src.id
