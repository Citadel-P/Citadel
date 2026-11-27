SELECT container.dockernodeid,container.id,latest.created FROM members container
LEFT JOIN LATERAL (
    SELECT latest.created FROM statisticssources src
    CROSS JOIN LATERAL (
        -- Keep sourceid in the ordering to use the composite index on misses.
        (SELECT created FROM statisticssamples
         WHERE sourceid>=src.id AND sourceid<=src.id AND created<=$3
         ORDER BY sourceid DESC,created DESC LIMIT 1)
        UNION ALL
        (SELECT created FROM statisticsrollups
         WHERE sourceid>=src.id AND sourceid<=src.id AND created<=$3
         ORDER BY sourceid DESC,created DESC LIMIT 1)
        ORDER BY created DESC LIMIT 1
    ) latest WHERE src.containerid=container.id OFFSET 0
) latest ON true
