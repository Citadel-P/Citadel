-- Filter the requested range before grouping; fetch full JSON only for representatives.
-- Recent observations and all retained transitions stay exact. Older periodic
-- samples are grouped by state/evidence, so healthy samples never hide failures.
WITH retained AS NOT MATERIALIZED (
    SELECT id,observedat,state,health,origin FROM containerhealthtransitions
    WHERE containerid=$1 AND recordedat>=to_timestamp($4::bigint)-interval '7 days'
      AND observedat>=GREATEST($2,$4-604800)
      AND observedat<=LEAST($3,$4)
), samples AS (
    SELECT id,observedat,state,health->'configuration' AS configuration,health->'result' AS result,health->'oomKilled' AS oom,health->'exitCode' AS exit,
        CASE WHEN observedat>=($4-3600) THEN id
             ELSE observedat / CASE WHEN observedat>=($4-86400) THEN 300 ELSE 1800 END
        END AS bucket,
        observedat>=($4-3600) AS recent
    FROM retained WHERE origin='Sample'
), grouped AS (
    SELECT recent,bucket,state,configuration,result,oom,exit,count(*) AS samplecount,
        min(observedat) AS firstobservedat,max(observedat) AS observedat,
        (array_agg(id ORDER BY observedat DESC,id DESC))[1] AS id
    FROM samples GROUP BY recent,bucket,state,configuration,result,oom,exit
)
SELECT id,observedat,state,health,origin,1::bigint AS samplecount,observedat AS firstobservedat
FROM retained WHERE origin<>'Sample'
UNION ALL
SELECT g.id,g.observedat,g.state,h.health,h.origin,g.samplecount,g.firstobservedat FROM grouped g JOIN containerhealthtransitions h ON h.id=g.id
ORDER BY observedat DESC,id DESC;
