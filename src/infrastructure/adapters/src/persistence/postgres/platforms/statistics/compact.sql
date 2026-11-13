-- One physical source, at most one hour. Platform lock fences capture retries.
WITH marked AS (
 SELECT *, CASE WHEN row_number() OVER w=1 OR ownerid IS DISTINCT FROM lag(ownerid) OVER w
     OR created/60<>lag(created/60) OVER w THEN 1 ELSE 0 END AS boundary
 FROM statisticssamples WHERE sourceid=$1 AND created>=$2 AND created<$3
 WINDOW w AS (ORDER BY created)
), grouped AS (
 SELECT *,sum(boundary) OVER (ORDER BY created) AS segment FROM marked
), samples AS (
 SELECT *,lag(created) OVER w AS previous_created,
 lag(rxbytes) OVER w AS previous_rxbytes,
 lag(txbytes) OVER w AS previous_txbytes,
 lag(blockreadbytes) OVER w AS previous_blockreadbytes,
 lag(blockwritebytes) OVER w AS previous_blockwritebytes
 FROM grouped WINDOW w AS (PARTITION BY segment ORDER BY created)
), deltas AS (
 SELECT *,
 CASE WHEN previous_rxbytes IS NULL OR rxbytes IS NULL THEN NULL WHEN rxbytes>=previous_rxbytes THEN rxbytes-previous_rxbytes ELSE rxbytes END AS delta_rxbytes,
 CASE WHEN previous_txbytes IS NULL OR txbytes IS NULL THEN NULL WHEN txbytes>=previous_txbytes THEN txbytes-previous_txbytes ELSE txbytes END AS delta_txbytes,
 CASE WHEN previous_blockreadbytes IS NULL OR blockreadbytes IS NULL THEN NULL WHEN blockreadbytes>=previous_blockreadbytes THEN blockreadbytes-previous_blockreadbytes ELSE blockreadbytes END AS delta_blockreadbytes,
 CASE WHEN previous_blockwritebytes IS NULL OR blockwritebytes IS NULL THEN NULL WHEN blockwritebytes>=previous_blockwritebytes THEN blockwritebytes-previous_blockwritebytes ELSE blockwritebytes END AS delta_blockwritebytes
 FROM samples
), summaries AS (
 SELECT sourceid,ownerid,segment,min(created) AS created,max(created) AS lastcreated,
 count(*)::integer AS samplecount,count(blockreadbytes)::integer AS readcount,count(blockwritebytes)::integer AS writecount,
 avg(cpuusage) AS cpuusage,
 avg(memoryactive) AS memoryactive,
 CASE WHEN count(memoryactivecache)=count(*) THEN avg(memoryactivecache) END AS memoryactivecache,
 avg(memorycache) AS memorycache,
 avg(memorylimit) AS memorylimit,
 avg(rxbytes) AS rxbytes,
 avg(txbytes) AS txbytes,
 avg(blockreadbytes) AS blockreadbytes,
 avg(blockwritebytes) AS blockwritebytes,
 (array_agg(rxbytes ORDER BY created))[1] AS first_rxbytes,(array_agg(rxbytes ORDER BY created DESC))[1] AS last_rxbytes,
 sum(delta_rxbytes) AS delta_rxbytes,COALESCE(sum(created-previous_created) FILTER(WHERE delta_rxbytes IS NOT NULL),0)::bigint AS duration_rxbytes,
 (array_agg(txbytes ORDER BY created))[1] AS first_txbytes,(array_agg(txbytes ORDER BY created DESC))[1] AS last_txbytes,
 sum(delta_txbytes) AS delta_txbytes,COALESCE(sum(created-previous_created) FILTER(WHERE delta_txbytes IS NOT NULL),0)::bigint AS duration_txbytes,
 (array_agg(blockreadbytes ORDER BY created))[1] AS first_blockreadbytes,(array_agg(blockreadbytes ORDER BY created DESC))[1] AS last_blockreadbytes,
 sum(delta_blockreadbytes) AS delta_blockreadbytes,COALESCE(sum(created-previous_created) FILTER(WHERE delta_blockreadbytes IS NOT NULL),0)::bigint AS duration_blockreadbytes,
 (array_agg(blockwritebytes ORDER BY created))[1] AS first_blockwritebytes,(array_agg(blockwritebytes ORDER BY created DESC))[1] AS last_blockwritebytes,
 sum(delta_blockwritebytes) AS delta_blockwritebytes,COALESCE(sum(created-previous_created) FILTER(WHERE delta_blockwritebytes IS NOT NULL),0)::bigint AS duration_blockwritebytes,
 ARRAY[max(cpuusage),max(memoryactive),max(memorycache),max(memorylimit)] AS peaks
 FROM deltas GROUP BY sourceid,ownerid,segment HAVING count(*)>=4
), inserted AS (
 INSERT INTO statisticsrollups(sourceid,ownerid,created,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes,
 samplecount,readcount,writecount,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations,peaks)
 SELECT sourceid,ownerid,created,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes,
 samplecount,readcount,writecount,lastcreated,
 ARRAY[first_rxbytes,first_txbytes,first_blockreadbytes,first_blockwritebytes],
 ARRAY[last_rxbytes,last_txbytes,last_blockreadbytes,last_blockwritebytes],
 ARRAY[delta_rxbytes,delta_txbytes,delta_blockreadbytes,delta_blockwritebytes],
 ARRAY[duration_rxbytes,duration_txbytes,duration_blockreadbytes,duration_blockwritebytes],peaks FROM summaries
 RETURNING sourceid,created,lastcreated
), removed AS (
 DELETE FROM statisticssamples s USING inserted r
 -- Repeat the input bounds so PostgreSQL can prune unrelated daily partitions
 -- and use the source/time index instead of scanning retained history per batch.
 WHERE s.sourceid=$1 AND s.created>=$2 AND s.created<$3
 AND s.sourceid=r.sourceid AND s.created>=r.created AND s.created<=r.lastcreated RETURNING s.created
), sealed AS (
 UPDATE statisticssources SET compacteduntil=$3 WHERE id=$1 RETURNING id
)
SELECT count(*)::bigint FROM removed
