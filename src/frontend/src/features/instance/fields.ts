import type { InstanceConfigurationView } from '@/api/generated/api.types';

export type InstanceField = {
  key: keyof InstanceConfigurationView;
  label: string;
  environment: string;
  description: string;
  minimum?: number;
  maximum?: number;
  choices?: string[];
};
export const instanceSections: { label: string; description: string; fields: InstanceField[] }[] = [
  {
    label: 'Security',
    description: 'Sign-in policies, session duration and service-account access.',
    fields: [
      {
        key: 'passwordMinimumLength',
        label: 'Minimum password length (characters)',
        environment: 'CITADEL_PASSWORDS_MINIMUM_LENGTH',
        minimum: 8,
        maximum: 128,
        description: 'Minimum length for new or changed passwords. Existing passwords are unaffected.',
      },
      {
        key: 'browserSessionLifetimeDays',
        label: 'Browser session lifetime (days)',
        environment: 'CITADEL_JWT_REFRESH_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description: 'Absolute lifetime of new browser sessions, not an idle timeout.',
      },
      {
        key: 'serviceAccountDefaultTokenLifetimeDays',
        label: 'Default service-account token lifetime (days)',
        environment: 'CITADEL_SERVICE_ACCOUNTS_DEFAULT_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description: 'Default expiry for new tokens. Must not exceed the maximum.',
      },
      {
        key: 'serviceAccountMaximumTokenLifetimeDays',
        label: 'Maximum expiring service-account token lifetime (days)',
        environment: 'CITADEL_SERVICE_ACCOUNTS_MAXIMUM_TOKEN_LIFETIME_DAYS',
        minimum: 1,
        maximum: 365000,
        description: 'Expiry limit for new expiring tokens. Existing tokens are unchanged.',
      },
      {
        key: 'serviceAccountMaximumActiveTokens',
        label: 'Maximum active tokens per service account',
        environment: 'CITADEL_SERVICE_ACCOUNTS_MAXIMUM_ACTIVE_TOKENS_PER_ACCOUNT',
        minimum: 1,
        maximum: 2147483647,
        description: 'Limit checked when creating tokens. Existing tokens are not revoked.',
      },
      {
        key: 'accessTokenLifetimeMinutes',
        label: 'Access-token lifetime (minutes)',
        environment: 'CITADEL_JWT_ACCESS_TOKEN_LIFETIME_MINUTES',
        minimum: 1,
        maximum: 2147483647,
        description: 'Lifetime of newly issued and refreshed access tokens.',
      },
    ],
  },
  {
    label: 'Monitoring',
    description: 'Balance statistics detail with collection and storage load.',
    fields: [
      {
        key: 'statisticsCollectionIntervalSeconds',
        label: 'Statistics collection interval (seconds)',
        environment: 'CITADEL_STATS_POLL_INTERVAL_SECONDS',
        minimum: 1,
        maximum: 2147483647,
        description: 'Time between samples across all platforms. Shorter intervals increase load.',
      },
      {
        key: 'statisticsWriteIntervalSeconds',
        label: 'Statistics write interval (seconds)',
        environment: 'CITADEL_STATS_FLUSH_INTERVAL_SECONDS',
        minimum: 1,
        maximum: 2147483647,
        description: 'Maximum buffering time before writing samples. Longer intervals delay alerts.',
      },
    ],
  },
  {
    label: 'Scripts',
    description: 'Execution capacity and timeout defaults for scripts.',
    fields: [
      {
        key: 'scriptsEnabled',
        label: 'Allow script execution',
        environment: 'CITADEL_SCRIPTS_ENABLED',
        description: 'Allow scripts on Core and Agents. Running work is unaffected.',
      },
      {
        key: 'scriptsMaximumParallelRuns',
        label: 'Maximum concurrent scripts on Core',
        environment: 'CITADEL_SCRIPTS_MAX_PARALLEL_RUNS',
        minimum: 1,
        maximum: 64,
        description: 'Local execution limit, independent of Agent capacity.',
      },
      {
        key: 'scriptsMaximumRemoteRuns',
        label: 'Maximum remote script runs tracked by Core',
        environment: 'CITADEL_SCRIPTS_MAX_REMOTE_RUNS',
        minimum: 1,
        maximum: 256,
        description: 'Remote runs Core can track. Agents enforce their own execution limits.',
      },
      {
        key: 'scriptsDefaultTimeoutSeconds',
        label: 'Default script timeout (seconds)',
        environment: 'CITADEL_SCRIPTS_DEFAULT_TIMEOUT_SECONDS',
        minimum: 1,
        maximum: 86400,
        description: 'Default for new scripts. Must not exceed the maximum.',
      },
      {
        key: 'scriptsMaximumTimeoutSeconds',
        label: 'Maximum script timeout (seconds)',
        environment: 'CITADEL_SCRIPTS_MAX_TIMEOUT_SECONDS',
        minimum: 1,
        maximum: 86400,
        description: 'Longest allowed timeout for script configuration and execution.',
      },
    ],
  },
  {
    label: 'Builds',
    description: 'Build preparation, tracking capacity and history retention.',
    fields: [
      {
        key: 'buildPreparationSlots',
        label: 'Concurrent build preparations',
        environment: 'CITADEL_BUILDS_PREPARATION_SLOTS',
        minimum: 1,
        maximum: 16,
        description: 'Concurrent Git checkout and build-context preparation tasks.',
      },
      {
        key: 'buildTrackingSlots',
        label: 'Maximum active builds tracked by Core',
        environment: 'CITADEL_BUILDS_TRACKING_SLOTS',
        minimum: 1,
        maximum: 128,
        description: 'Builds Core can track. Build Pools and Agents keep their own limits.',
      },
      {
        key: 'buildCleanupEnabled',
        label: 'Automatically clean up completed build history',
        environment: 'CITADEL_BUILDS_RUN_CLEANUP_ENABLED',
        description: 'Remove eligible completed runs. Active and referenced runs are protected.',
      },
      {
        key: 'buildRetentionDays',
        label: 'Keep completed build history for (days)',
        environment: 'CITADEL_BUILDS_RUN_RETENTION_DAYS',
        minimum: 1,
        maximum: 2147483647,
        description: 'Age at which completed runs become eligible for cleanup, when enabled.',
      },
    ],
  },
  {
    label: 'Backups',
    description: 'Worker availability, operation capacity and timeouts.',
    fields: [
      {
        key: 'backupsEnabled',
        label: 'Enable backup and restore workers',
        environment: 'CITADEL_BACKUPS_ENABLED',
        description: 'Enable scheduling and dispatch. When disabled, manual requests may remain queued.',
      },
      {
        key: 'backupTrackingSlots',
        label: 'Maximum active backup, restore and repository operations',
        environment: 'CITADEL_BACKUPS_TRACKING_SLOTS',
        minimum: 1,
        maximum: 128,
        description: 'Shared tracking limit for backup, restore and repository operations.',
      },
      {
        key: 'backupDefaultTimeoutSeconds',
        label: 'Repository operation and restore timeout (seconds)',
        environment: 'CITADEL_BACKUPS_DEFAULT_TIMEOUT_SECONDS',
        minimum: 5,
        maximum: 86400,
        description: 'Timeout for restore and repository work. Backup Policies set their own backup timeout.',
      },
    ],
  },
  {
    label: 'Runtime',
    description: 'API documentation, Agent connections and Core logging.',
    fields: [
      {
        key: 'openapiEnabled',
        label: 'Serve Swagger UI and OpenAPI documents',
        environment: 'CITADEL_HTTP_OPENAPI_ENABLED',
        description: 'Expose API documentation. The REST API stays available.',
      },
      {
        key: 'requireDirectAgentHttps',
        label: 'Require HTTPS for Direct Agents',
        environment: 'CITADEL_CORE_AGENT_ALLOW_INSECURE',
        description: 'Reject HTTP Direct Agent connections. Inverse of the allow-insecure environment flag.',
      },
      {
        key: 'agentImageOverride',
        label: 'Agent image override',
        environment: 'CITADEL_EDGE_AGENT_IMAGE',
        description: 'Leave empty to match Core automatically. Does not upgrade running Agents.',
      },
      {
        key: 'logVerbosity',
        label: 'Log verbosity',
        environment: 'RUST_LOG',
        choices: ['off', 'error', 'warn', 'info', 'debug', 'trace'],
        description: 'Core logging detail. Deployment module filters remain managed externally.',
      },
      {
        key: 'logFormat',
        label: 'Log output format',
        environment: 'CITADEL_LOG_FORMAT',
        choices: ['Text', 'Json'],
        description: 'Choose plain text or structured JSON for Core logs.',
      },
    ],
  },
];
