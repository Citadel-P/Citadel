SELECT version FROM (
    SELECT agentversion AS version FROM platforms WHERE connectortype <> 'Local'
    UNION ALL
    SELECT lastseenversion AS version FROM edgeagentbindings WHERE revokedatutc IS NULL
    UNION ALL
    -- A managed Swarm installation that has no observed binding still needs
    -- compatibility review, even when its first Agent has not connected yet.
    SELECT NULL::text AS version FROM swarmnodeagentinstallations i
    WHERE i.desiredstate = 'Installed' AND NOT EXISTS (
        SELECT 1 FROM edgeagentbindings b
        WHERE b.platformid = i.platformid AND b.profile = 'SwarmNode' AND b.revokedatutc IS NULL
    )
) agents
LIMIT 4097
