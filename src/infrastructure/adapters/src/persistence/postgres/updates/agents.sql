SELECT version FROM (
    SELECT agentversion AS version FROM platforms WHERE connectortype = 'Agent'
    UNION ALL
    -- The authenticated binding is authoritative for Edge installations, not the
    -- ordinary Platform projection (which can retain older metadata).
    SELECT NULL::text FROM platforms p WHERE connectortype = 'EdgeAgent'
    AND NOT EXISTS (
        SELECT 1 FROM edgeagentbindings b WHERE b.platformid = p.id
        AND b.profile = 'Ordinary' AND b.revokedatutc IS NULL
    )
    UNION ALL
    SELECT lastseenversion FROM edgeagentbindings b WHERE revokedatutc IS NULL
    AND (b.resourcetype <> 'BuildAgentPool' OR EXISTS (
        SELECT 1 FROM buildagentpools p WHERE p.id = b.resourceid
        AND p.enabled AND p.archivedat IS NULL
    ))
    UNION ALL
    SELECT CASE WHEN COALESCE(p.providerspec->>'connectionMode', p.providerspec->>'ConnectionMode', 'InboundAgent') = 'InboundAgent'
        THEN p.agentversion ELSE NULL::text END
    FROM buildagentpools p WHERE p.provider = 'SelfManagedVm' AND p.enabled AND p.archivedat IS NULL
    AND NOT EXISTS (
        SELECT 1 FROM edgeagentbindings b WHERE b.resourceid = p.id
        AND b.resourcetype = 'BuildAgentPool' AND b.revokedatutc IS NULL
        AND COALESCE(p.providerspec->>'connectionMode', p.providerspec->>'ConnectionMode') = 'EdgeAgent'
    )
    UNION ALL
    -- Desired installations still matter before first enrollment.
    SELECT NULL::text FROM swarmnodeagentinstallations i
    WHERE i.desiredstate = 'Installed' AND NOT EXISTS (
        SELECT 1 FROM edgeagentbindings b
        WHERE b.platformid = i.platformid AND b.profile = 'SwarmNode' AND b.revokedatutc IS NULL
    )
) agents
LIMIT 4097
