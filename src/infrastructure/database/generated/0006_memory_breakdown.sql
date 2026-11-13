-- Existing observations have no active-cache measurement: preserve NULL rather than inventing zero.
ALTER TABLE statisticssamples ADD COLUMN memoryactivecache double precision;
ALTER TABLE statisticsrollups ADD COLUMN memoryactivecache double precision;

CREATE OR REPLACE VIEW statisticsobservations AS
SELECT s.sourceid,s.ownerid,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,1::integer AS samplecount,
    (blockreadbytes IS NOT NULL)::integer AS readcount,
    (blockwritebytes IS NOT NULL)::integer AS writecount,
    created AS lastcreated,
    ARRAY[rxbytes,txbytes,blockreadbytes,blockwritebytes] AS firstcounters,
    ARRAY[rxbytes,txbytes,blockreadbytes,blockwritebytes] AS lastcounters,
    ARRAY[NULL,NULL,NULL,NULL]::double precision[] AS counterdeltas,
    ARRAY[0,0,0,0]::bigint[] AS counterdurations,
    ARRAY[cpuusage,memoryactive,memorycache,memorylimit] AS peaks,s.memoryactivecache FROM statisticssamples s
UNION ALL SELECT * FROM statisticsrollups;
CREATE OR REPLACE VIEW containerstats AS
SELECT src.containerid,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks,s.memoryactivecache
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN containers c ON c.id=src.containerid;
CREATE OR REPLACE VIEW workloadstats AS
SELECT src.platformid,o.deploymentid,o.stackid,o.servicename,o.replicakey,src.containerid,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks,s.memoryactivecache
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN statisticsowners o ON o.id=s.ownerid AND o.kind='Workload';
CREATE OR REPLACE VIEW swarmservicestats AS
SELECT src.platformid,o.dockerserviceid,o.dockertaskid,o.servicename,o.servicestackid AS stackid,o.swarmserviceid,o.replicakey AS taskkey,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks,s.memoryactivecache
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN statisticsowners o ON o.id=s.ownerid AND o.kind='Service';
