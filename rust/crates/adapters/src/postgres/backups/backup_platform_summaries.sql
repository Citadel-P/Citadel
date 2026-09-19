, requested AS (
    SELECT DISTINCT unnest($5::uuid[]) AS platformid
), policies AS (
    SELECT p.id, p.enabled, p.source->>'$type' AS sourcetype,
        CASE p.source->>'$type'
            WHEN 'DockerVolume' THEN NULLIF(COALESCE(p.source->>'PlatformId',p.source->>'platformId'),'')::uuid
            WHEN 'Stack' THEN sr.platformid
            WHEN 'Deployment' THEN d.platformid
            WHEN 'SwarmService' THEN ss.platformid
        END AS platformid
    FROM backuppolicies p
    LEFT JOIN stacks s ON p.source->>'$type'='Stack'
        AND s.id=NULLIF(COALESCE(p.source->>'StackId',p.source->>'stackId'),'')::uuid
    LEFT JOIN stackreleases sr ON sr.id=s.currentstackreleaseid
    LEFT JOIN deployments d ON p.source->>'$type'='Deployment'
        AND d.id=NULLIF(COALESCE(p.source->>'DeploymentId',p.source->>'deploymentId'),'')::uuid
    LEFT JOIN swarmservices ss ON p.source->>'$type'='SwarmService'
        AND ss.id=NULLIF(COALESCE(p.source->>'SwarmServiceId',p.source->>'swarmServiceId'),'')::uuid
    WHERE p.archivedat IS NULL
      AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
          SELECT 1 FROM actor_scope scope
          JOIN resourceaccesses access ON access.actorid=scope.actorid
          WHERE access.resourcetype=$2 AND access.resourceid=p.id
            AND access.permissionlevel = ANY($3)
      ))
), platform_policies AS (
    SELECT p.* FROM policies p JOIN requested r ON r.platformid=p.platformid
), latest_policy_runs AS (
    SELECT DISTINCT ON (p.id) p.id AS policyid, p.platformid, r.status,
        COALESCE(r.completedat,r.queuedat) AS runat
    FROM platform_policies p JOIN backupruns r ON r.backuppolicyid=p.id
    ORDER BY p.id,r.queuedat DESC,r.id DESC
), latest_platform_runs AS (
    SELECT DISTINCT ON (platformid) platformid,status,runat FROM latest_policy_runs
    ORDER BY platformid,runat DESC,policyid DESC
)
SELECT requested.platformid,
    count(p.id)::int AS policycount,
    count(p.id) FILTER (WHERE p.enabled)::int AS enabledpolicycount,
    count(p.id) FILTER (WHERE p.sourcetype='DockerVolume')::int AS dockervolumepolicycount,
    count(p.id) FILTER (WHERE p.sourcetype='Stack')::int AS stackpolicycount,
    count(p.id) FILTER (WHERE p.sourcetype='Deployment')::int AS deploymentpolicycount,
    count(p.id) FILTER (WHERE p.sourcetype='SwarmService')::int AS swarmservicepolicycount,
    count(p.id) FILTER (WHERE latest.status IN ('Failed','TimedOut','Interrupted'))::int AS attentionpolicycount,
    latest_platform.status AS lastrunstatus, latest_platform.runat AS lastrunat
FROM requested
LEFT JOIN platform_policies p ON p.platformid=requested.platformid
LEFT JOIN latest_policy_runs latest ON latest.policyid=p.id
LEFT JOIN latest_platform_runs latest_platform ON latest_platform.platformid=requested.platformid
GROUP BY requested.platformid,latest_platform.status,latest_platform.runat
ORDER BY requested.platformid
