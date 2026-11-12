-- Inspect one complete minute, at most 60 timestamps, without record expansion.
-- Text size bounds uncompressed JSON, including unusually long historical IDs.
SELECT count(*)::bigint,COALESCE(sum(jsonb_array_length(containerio)),0)::bigint,
    COALESCE(sum(octet_length(containerio::text)),0)::bigint FROM (
    SELECT s.containerio FROM platformstats s JOIN platformhistorysources src
        ON src.platformid=s.platformid AND src.nodeid='' WHERE src.id=$1 AND s.created>=$2 AND s.created<$3
    UNION ALL
    SELECT s.containerio FROM platformnodeiostats s JOIN platformhistorysources src
        ON src.platformid=s.platformid AND src.nodeid=s.nodeid AND src.nodeid<>''
        WHERE src.id=$1 AND s.created>=$2 AND s.created<$3
) captures
