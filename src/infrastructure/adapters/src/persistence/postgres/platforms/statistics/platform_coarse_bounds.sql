-- Only ranges already sealed by minute compaction are immutable. Seek to the
-- first retained minute row, then process one complete fifteen-minute block.
SELECT src.id,src.platformid,first.created/900*900,first.created/900*900+900
FROM platformhistorysources src
CROSS JOIN LATERAL (
    SELECT created FROM platformiorollups
    WHERE sourceid>=src.id AND sourceid<=src.id
        AND created>=GREATEST(src.coarseneduntil,($1+899)/900*900)
        AND created<src.compacteduntil/900*900
    ORDER BY sourceid,created LIMIT 1
) first
WHERE ($2::bigint IS NULL OR src.id=$2)
ORDER BY (src.id<=$3),src.id LIMIT 1
