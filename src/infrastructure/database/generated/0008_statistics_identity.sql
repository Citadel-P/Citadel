-- Reconcile verified host/node projections of the same physical Swarm Task.
CREATE OR REPLACE FUNCTION statistics_source(p uuid, c uuid, t text) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE existing_source statisticssources; result bigint;
BEGIN
    SELECT * INTO existing_source FROM statisticssources
    WHERE platformid=p AND ((c IS NOT NULL AND containerid=c) OR (t IS NOT NULL AND dockertaskid=t))
    ORDER BY (dockertaskid IS NOT DISTINCT FROM t) DESC LIMIT 1;
    IF FOUND THEN
        -- Host/node inventory can keep two projections of one task during a
        -- same-cluster manager handover. Validate its physical identity against
        -- the persisted Task before attaching the current projection. Deleted
        -- projections can be rediscovered under the same checks. A former host's
        -- NULL node belongs to its already associated Task, not the manager in
        -- today's descriptor; the explicit node of a node projection must match.
        IF c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid
            AND t IS NOT NULL AND t=existing_source.dockertaskid
            AND EXISTS (
                SELECT 1 FROM containers next
                JOIN platforms platform ON platform.id=next.platformid
                JOIN swarmtaskprojections task ON task.platformid=platform.id AND task.dockertaskid=t
                LEFT JOIN containers previous ON previous.id=existing_source.containerid
                WHERE next.id=c AND next.platformid=p
                    AND next.dockercontainerid=task.dockercontainerid
                    AND COALESCE(next.dockernodeid,platform.platformdescriptor->>'nodeID')=task.dockernodeid
                    AND (previous.id IS NULL OR (
                        previous.platformid=p AND previous.dockercontainerid=next.dockercontainerid
                        AND (previous.dockernodeid IS NULL OR previous.dockernodeid=task.dockernodeid)))
            ) THEN
            UPDATE statisticssources SET containerid=c WHERE id=existing_source.id;
            existing_source.containerid := c;
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid)
            THEN
            RAISE EXCEPTION 'Statistics physical identity changed';
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NULL) OR (t IS NOT NULL AND t IS DISTINCT FROM existing_source.dockertaskid) THEN
            UPDATE statisticssources SET containerid=COALESCE(containerid,c),dockertaskid=COALESCE(t,dockertaskid) WHERE id=existing_source.id;
        END IF;
        RETURN existing_source.id;
    END IF;
    INSERT INTO statisticssources(platformid,containerid,dockertaskid) VALUES(p,c,t) RETURNING id INTO result;
    RETURN result;
END;
$$;
