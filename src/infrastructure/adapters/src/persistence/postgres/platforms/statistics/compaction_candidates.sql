-- Bound input before probing history; empty/churned identities advance the cursor too.
WITH page AS MATERIALIZED (
    SELECT id,platformid,compacteduntil FROM statisticssources
    WHERE id>$3 ORDER BY id LIMIT 64
)
SELECT src.id,src.platformid,COALESCE(first.created/60<last.created/60,false) AS eligible
FROM page src
LEFT JOIN LATERAL (
    -- Range bounds retain sourceid in the ordering, enforcing a composite-index
    -- seek even when most sources have no samples. Equality can cause PostgreSQL
    -- to use the timestamp index and filter the entire history for each miss.
    SELECT created FROM statisticssamples
    WHERE sourceid>=src.id AND sourceid<=src.id
        AND created>=GREATEST(src.compacteduntil,$2) AND created<$1
    ORDER BY sourceid,created LIMIT 1
) first ON true
LEFT JOIN LATERAL (
    SELECT created FROM statisticssamples
    WHERE first.created IS NOT NULL AND sourceid>=src.id AND sourceid<=src.id
    ORDER BY sourceid DESC,created DESC LIMIT 1
) last ON first.created IS NOT NULL
ORDER BY src.id
