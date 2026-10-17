-- Recent observations and all retained transitions stay exact. Older periodic
-- samples are grouped by state/evidence, so healthy samples never hide failures.
WITH retained AS MATERIALIZED (
    SELECT * FROM containerhealthtransitions
    WHERE containerid=$1 AND recordedat>=now()-interval '7 days'
      AND observedat>=extract(epoch FROM now()-interval '7 days')::bigint
      AND observedat<=extract(epoch FROM now())::bigint
), samples AS (
    SELECT *,
        CASE WHEN observedat>=extract(epoch FROM now()-interval '1 hour')::bigint THEN id
             ELSE observedat / CASE WHEN observedat>=extract(epoch FROM now()-interval '1 day')::bigint THEN 300 ELSE 1800 END
        END AS bucket,
        observedat>=extract(epoch FROM now()-interval '1 hour')::bigint AS recent
    FROM retained WHERE origin='Sample'
), grouped AS (
    SELECT *,
        count(*) OVER bucket_group AS samplecount,
        min(observedat) OVER bucket_group AS firstobservedat,
        row_number() OVER (bucket_group ORDER BY observedat DESC, id DESC) AS position
    FROM samples
    WINDOW bucket_group AS (PARTITION BY recent,bucket,state,health->'configuration',health->'result',health->'oomKilled',health->'exitCode')
)
SELECT id,observedat,state,health,origin,1::bigint AS samplecount,observedat AS firstobservedat
FROM retained WHERE origin<>'Sample'
UNION ALL
SELECT id,observedat,state,health,origin,samplecount,firstobservedat FROM grouped WHERE position=1
ORDER BY observedat DESC,id DESC;
