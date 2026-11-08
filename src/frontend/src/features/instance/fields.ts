import type { InstanceConfigurationView } from '@/api/generated/api.types';

export type InstanceField = {
  key: keyof InstanceConfigurationView;
  label: string;
  environment: string;
  description: string;
  minimum?: number;
  maximum?: number;
  choices?: string[];
  advanced?: boolean;
};
export const instanceSections: { label: string; fields: InstanceField[] }[] = [
  {
    label: 'Security',
    fields: [
      {
        key: 'passwordMinimumLength',
        label: 'Minimum password length (characters)',
        environment: 'CITADEL_PASSWORDS_MINIMUM_LENGTH',
        minimum: 8,
        maximum: 128,
        description:
          'Applies to new and changed passwords, including administrator changes. Existing passwords still work; common-password checks remain enabled.',
      },
      {
        key: 'browserSessionLifetimeDays',
        label: 'Browser session lifetime (days)',
        environment: 'CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description:
          'Absolute lifetime for new sessions after restart, not an idle timeout. Existing session expirations stay unchanged.',
      },
      {
        key: 'serviceAccountDefaultTokenLifetimeDays',
        label: 'Default service-account token lifetime (days)',
        environment: 'CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description: 'Default for subsequently created tokens. Must not exceed the maximum.',
      },
      {
        key: 'serviceAccountMaximumTokenLifetimeDays',
        label: 'Maximum expiring service-account token lifetime (days)',
        environment: 'CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description:
          'Applies to new expiring tokens. Existing tokens are not shortened or revoked. The explicit non-expiring option and its entitlements remain unchanged.',
      },
      {
        key: 'serviceAccountMaximumActiveTokens',
        label: 'Maximum active tokens per service account',
        environment: 'CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT',
        minimum: 1,
        maximum: 2147483647,
        description: 'Checked when creating tokens. Reducing this limit does not revoke existing tokens.',
      },
      {
        key: 'accessTokenLifetimeMinutes',
        label: 'Access-token lifetime (minutes)',
        environment: 'CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES',
        minimum: 1,
        maximum: 2147483647,
        advanced: true,
        description: 'After restart, applies to subsequently issued access tokens, including refreshes.',
      },
    ],
  },
  {
    label: 'Monitoring',
    fields: [
      {
        key: 'statisticsCollectionIntervalSeconds',
        label: 'Statistics collection interval (seconds)',
        environment: 'CITADEL_STATS_POLL_INTERVAL_SECONDS',
        minimum: 1,
        maximum: 2147483647,
        description:
          'Applies to Local, Direct and Edge monitoring. Shorter intervals increase detail and collection/storage load. History retention and compaction stay unchanged.',
      },
      {
        key: 'statisticsWriteIntervalSeconds',
        label: 'Statistics write interval (seconds)',
        environment: 'CITADEL_STATS_FLUSH_INTERVAL_SECONDS',
        minimum: 1,
        maximum: 2147483647,
        advanced: true,
        description:
          'Controls buffering for both statistics writers; a full batch flushes earlier. Longer buffering delays committed data and alert processing. It does not reduce sample count or downsample history.',
      },
    ],
  },
  {
    label: 'Scripts',
    fields: [
      {
        key: 'scriptsEnabled',
        label: 'Allow script execution',
        environment: 'CITADEL_SCRIPTS_ENABLED',
        description:
          'Controls Script execution on Core and remote Agents. Existing running work keeps its established lifecycle.',
      },
      {
        key: 'scriptsMaximumParallelRuns',
        label: 'Maximum concurrent scripts on Core',
        environment: 'CITADEL_SCRIPTS_MAX_PARALLEL_RUNS',
        minimum: 1,
        maximum: 64,
        description: 'Core execution capacity; independent of remote Agent capacities.',
      },
      {
        key: 'scriptsMaximumRemoteRuns',
        label: 'Maximum remote script runs tracked by Core',
        environment: 'CITADEL_SCRIPTS_MAX_REMOTE_RUNS',
        minimum: 1,
        maximum: 256,
        description: 'Core tracking limit. Each Agent still limits its own execution capacity.',
      },
      {
        key: 'scriptsDefaultTimeoutSeconds',
        label: 'Default script timeout (seconds)',
        environment: 'CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS',
        minimum: 1,
        maximum: 86400,
        description:
          'Default for new Scripts and API requests omitting a timeout. Saved Scripts are not rewritten. Must not exceed the maximum.',
      },
      {
        key: 'scriptsMaximumTimeoutSeconds',
        label: 'Maximum script timeout (seconds)',
        environment: 'CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS',
        minimum: 1,
        maximum: 86400,
        description: 'Maximum accepted timeout for subsequent Script configuration and execution.',
      },
    ],
  },
  {
    label: 'Builds',
    fields: [
      {
        key: 'buildPreparationSlots',
        label: 'Concurrent build preparations',
        environment: 'CITADEL_BUILDS_PREPARATION_SLOTS',
        minimum: 1,
        maximum: 16,
        description: 'Concurrent Core Git and context preparation work.',
      },
      {
        key: 'buildTrackingSlots',
        label: 'Maximum active builds tracked by Core',
        environment: 'CITADEL_BUILDS_TRACKING_SLOTS',
        minimum: 1,
        maximum: 128,
        description: 'Core tracking capacity does not override Build Pool or Agent execution capacity.',
      },
      {
        key: 'buildCleanupEnabled',
        label: 'Automatically clean up completed build history',
        environment: 'CITADEL_BUILDS_RUN_CLEANUP_ENABLED',
        description:
          'Protects active, current, reserved, completion-pending, deployment-referenced and release-referenced runs.',
      },
      {
        key: 'buildRetentionDays',
        label: 'Keep completed build history for (days)',
        environment: 'CITADEL_BUILDS_RUN_RETENTION_DAYS',
        minimum: 1,
        maximum: 2147483647,
        description:
          'Retained while cleanup is disabled. Reducing this period makes more eligible completed history subject to deletion after restart.',
      },
    ],
  },
  {
    label: 'Backups',
    fields: [
      {
        key: 'backupsEnabled',
        label: 'Enable backup and restore workers',
        environment: 'CITADEL_BACKUPS_ENABLED',
        description:
          'Stops scheduled enqueueing and backup/restore dispatch when disabled. Manual requests may remain queued until workers are enabled and Core restarted. This is not a switch for every repository operation.',
      },
      {
        key: 'backupTrackingSlots',
        label: 'Maximum active backup, restore and repository operations',
        environment: 'CITADEL_BACKUPS_TRACKING_SLOTS',
        minimum: 1,
        maximum: 128,
        description: 'Combined tracking capacity for these operations.',
      },
      {
        key: 'backupDefaultTimeoutSeconds',
        label: 'Repository operation and restore timeout (seconds)',
        environment: 'CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS',
        minimum: 5,
        maximum: 86400,
        description: 'Backup Policies retain their own backup timeout and snapshot retention.',
      },
    ],
  },
  {
    label: 'Advanced',
    fields: [
      {
        key: 'openapiEnabled',
        label: 'Serve Swagger UI and OpenAPI documents',
        environment: 'CITADEL_HTTP_OPENAPI_ENABLED',
        description: 'Controls documentation routes. The application REST API remains available.',
      },
      {
        key: 'requireDirectAgentHttps',
        label: 'Require HTTPS for Direct Agents',
        environment: 'CITADEL_CORE_AGENT_ALLOW_INSECURE',
        description:
          'Inverse of the deployment allow-insecure flag. After restart, connections to existing HTTP Direct Agents are rejected; saved addresses are preserved. Certificate verification remains enabled. Does not configure browser TLS or the inbound Edge listener.',
      },
      {
        key: 'agentImageOverride',
        label: 'Agent image override',
        environment: 'CITADEL_EDGE_AGENT_IMAGE',
        description:
          'Leave empty for automatic version-matched resolution, including deployment image repository/tag options. Used by Direct/Edge/Swarm setup and helper-image consumers after restart. Does not upgrade running Agents.',
      },
      {
        key: 'logVerbosity',
        label: 'Log verbosity',
        environment: 'RUST_LOG',
        choices: ['off', 'error', 'warn', 'info', 'debug', 'trace'],
        description: 'Deployment module filters are preserved and managed through RUST_LOG.',
      },
      {
        key: 'logFormat',
        label: 'Log output format',
        environment: 'CITADEL_LOG_FORMAT',
        choices: ['Text', 'Json'],
        description: 'Controls the actual Core logger after restart. Terminal color remains deployment configuration.',
      },
    ],
  },
];
