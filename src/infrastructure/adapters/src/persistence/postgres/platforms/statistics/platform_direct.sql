WITH incoming AS (
    SELECT * FROM jsonb_to_recordset($2::jsonb) AS value(
        "dockerContainerId" text, "memoryActive" double precision,
        "memoryCache" double precision, "cpuUsage" double precision,
        "memoryLimit" double precision, "rxBytes" double precision,
        "txBytes" double precision, "blockReadBytes" double precision, "blockWriteBytes" double precision, created bigint)
), eligible AS (
    SELECT incoming.*, container.id AS persisted_container_id
    FROM incoming
    JOIN containers container
      ON container.platformid = $1
     AND container.dockercontainerid = incoming."dockerContainerId"
     AND container.dockernodeid IS NOT DISTINCT FROM $3
), platform_sample AS (
    SELECT COALESCE(SUM("memoryActive"), 0) AS memory_active,
           COALESCE(SUM("cpuUsage"), 0) AS cpu_usage,
           COALESCE(SUM("rxBytes"), 0) AS rx_bytes,
           COALESCE(SUM("txBytes"), 0) AS tx_bytes,
           jsonb_agg(jsonb_build_object('containerId',"dockerContainerId",
             'rxBytes',"rxBytes",'txBytes',"txBytes",'blockReadBytes',"blockReadBytes",'blockWriteBytes',"blockWriteBytes")) AS container_io,
           COALESCE(created, $7::bigint) AS created
    FROM eligible
    GROUP BY created
    UNION ALL SELECT 0,0,0,0,'[]'::jsonb,$7::bigint WHERE $7::bigint IS NOT NULL AND NOT EXISTS(SELECT 1 FROM eligible)
), sources AS (
    INSERT INTO platformhistorysources(platformid,nodeid)
    SELECT id,'' FROM platforms WHERE id=$1 AND $3::text IS NULL
        AND EXISTS(SELECT 1 FROM platform_sample WHERE created IS NOT NULL)
    ON CONFLICT(platformid,nodeid) DO NOTHING
), persisted_platform AS (
    INSERT INTO platformstats (
        id, platformid, memoryusage, cpuusage, rxbytes, txbytes, created, diskusedbytes, disktotalbytes, diskusage, alertpending, containerio)
    SELECT gen_random_uuid(), platform.id,
           CASE WHEN platform.memtotal > 0
                THEN sample.memory_active / platform.memtotal * 100.0 ELSE 0 END,
           CASE WHEN platform.cpucount > 0
                THEN sample.cpu_usage / platform.cpucount ELSE sample.cpu_usage END,
           sample.rx_bytes, sample.tx_bytes, sample.created, $4, $5, $6, true, sample.container_io
    FROM platform_sample sample
    JOIN platforms platform ON platform.id = $1
    WHERE sample.created IS NOT NULL AND $3::text IS NULL
      AND NOT EXISTS(SELECT 1 FROM platformhistorysources src
          WHERE src.platformid=$1 AND src.nodeid='' AND sample.created<src.compacteduntil)
    ON CONFLICT (platformid, created) DO UPDATE SET
        memoryusage = EXCLUDED.memoryusage,
        cpuusage = EXCLUDED.cpuusage,
        rxbytes = EXCLUDED.rxbytes,
        txbytes = EXCLUDED.txbytes,
        diskusedbytes = EXCLUDED.diskusedbytes,
        disktotalbytes = EXCLUDED.disktotalbytes,
        diskusage = EXCLUDED.diskusage,
        alertpending = true, containerio = EXCLUDED.containerio
)
SELECT COUNT(*) FROM eligible
