-- One sealed, aligned fifteen-minute block under the Platform lock. Retain the
-- original minute rows: partial ranges and finer charts still need them.
WITH source AS MATERIALIZED (
 SELECT * FROM platformhistorysources WHERE id=$1 AND compacteduntil>=$3
     AND coarseneduntil<=$2 AND $2%900=0 AND $3=$2+900
), candidates AS MATERIALIZED (
 -- Bound input before any sort/window expansion. Oversized optional blocks are
 -- skipped permanently, preserving minute history without retrying expensive work.
 SELECT r.* FROM platformiorollups r JOIN source src ON src.id=r.sourceid
 WHERE r.sourceid>=$1 AND r.sourceid<=$1 AND r.created>=$2 AND r.created<$3
 LIMIT 10001
), eligible AS (
 SELECT r.* FROM candidates r
 WHERE (SELECT count(*)<=10000 FROM candidates)
     AND NOT EXISTS(SELECT 1 FROM candidates WHERE lastcreated>=$3)
     -- Raw minutes rejected by the storage gate must not be bridged over by a
     -- coarse summary. Explicit bounds keep these checks on the current block.
     AND NOT EXISTS(SELECT 1 FROM platformstats raw JOIN source src ON src.platformid=raw.platformid
         WHERE src.nodeid='' AND raw.created>=$2 AND raw.created<$3)
     AND NOT EXISTS(SELECT 1 FROM platformnodeiostats raw JOIN source src ON src.platformid=raw.platformid AND src.nodeid=raw.nodeid
         WHERE src.nodeid<>'' AND raw.created>=$2 AND raw.created<$3)
), samples AS (
   SELECT *,lag(lastcreated) OVER w AS previous_created,lag(lastcounters) OVER w AS previous_counters FROM eligible
   WINDOW w AS (PARTITION BY sourceid,containerid,created/900 ORDER BY created)
  ), written AS (
 INSERT INTO platformiocoarserollups(sourceid,containerid,created,lastcreated,
 firstcounters,lastcounters,counterdeltas,counterdurations,blockreadbytes,blockwritebytes,readcount,writecount)
 SELECT sourceid,containerid,min(created),max(lastcreated),
  ARRAY[(array_agg(firstcounters[1] ORDER BY created ASC))[1],(array_agg(firstcounters[2] ORDER BY created ASC))[1],(array_agg(firstcounters[3] ORDER BY created ASC))[1],(array_agg(firstcounters[4] ORDER BY created ASC))[1]],
ARRAY[(array_agg(lastcounters[1] ORDER BY created DESC))[1],(array_agg(lastcounters[2] ORDER BY created DESC))[1],(array_agg(lastcounters[3] ORDER BY created DESC))[1],(array_agg(lastcounters[4] ORDER BY created DESC))[1]],
ARRAY[statistics_sum(sum(counterdeltas[1]),sum(statistics_delta(firstcounters[1],previous_counters[1]))),statistics_sum(sum(counterdeltas[2]),sum(statistics_delta(firstcounters[2],previous_counters[2]))),statistics_sum(sum(counterdeltas[3]),sum(statistics_delta(firstcounters[3],previous_counters[3]))),statistics_sum(sum(counterdeltas[4]),sum(statistics_delta(firstcounters[4],previous_counters[4])))],
ARRAY[sum(coalesce(counterdurations[1],0)+CASE WHEN firstcounters[1] IS NOT NULL AND previous_counters[1] IS NOT NULL THEN created-previous_created ELSE 0 END)::bigint,sum(coalesce(counterdurations[2],0)+CASE WHEN firstcounters[2] IS NOT NULL AND previous_counters[2] IS NOT NULL THEN created-previous_created ELSE 0 END)::bigint,sum(coalesce(counterdurations[3],0)+CASE WHEN firstcounters[3] IS NOT NULL AND previous_counters[3] IS NOT NULL THEN created-previous_created ELSE 0 END)::bigint,sum(coalesce(counterdurations[4],0)+CASE WHEN firstcounters[4] IS NOT NULL AND previous_counters[4] IS NOT NULL THEN created-previous_created ELSE 0 END)::bigint],sum(blockreadbytes*readcount)/nullif(sum(readcount),0),sum(blockwritebytes*writecount)/nullif(sum(writecount),0),sum(readcount)::integer,sum(writecount)::integer
  FROM samples GROUP BY sourceid,containerid,created/900
)
UPDATE platformhistorysources SET coarseneduntil=$3 WHERE id IN (SELECT id FROM source)
