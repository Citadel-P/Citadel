import {
  AutomationWebhookConfig,
  BackupWebhookConfig,
  BuildWebhookConfig,
  RepoWebhookConfig,
  StackWebhookConfig,
  SwarmServiceWebhookConfig,
  WebhookAuthScheme,
  WebhookProvider,
} from '@/api/generated/api.types';
import { FieldInput, FieldSelect, FieldSwitch } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { cn } from '@/lib/utils';
import { CheckCheck, Clipboard, KeyRound } from 'lucide-react';
import { useState } from 'react';

type WebhookConfigValue =
  | RepoWebhookConfig
  | StackWebhookConfig
  | AutomationWebhookConfig
  | BackupWebhookConfig
  | BuildWebhookConfig
  | SwarmServiceWebhookConfig;
type WebhookCommonConfig = Pick<RepoWebhookConfig, 'enabled' | 'provider' | 'authScheme' | 'secret' | 'branchFilter'>;
type NormalizedWebhookConfig = Required<Pick<WebhookCommonConfig, 'enabled' | 'provider' | 'authScheme'>> &
  Pick<WebhookCommonConfig, 'secret' | 'branchFilter'>;

type WebhookConfigFieldProps = {
  resourceType: 'repo' | 'stack' | 'automation-action' | 'backup-policy' | 'build' | 'swarm-service';
  resourceId?: string;
  execution: 'pull' | 'deploy' | 'run' | 'update';
  value?: WebhookConfigValue | null;
  defaultBranch?: string | null;
  showBranchFilter?: boolean;
  disabled?: boolean;
  enableDisabled?: boolean;
  onChange: (value: WebhookConfigValue) => void;
};

const providerLabels: Record<WebhookProvider, string> = {
  [WebhookProvider.GitHub]: 'GitHub',
  [WebhookProvider.GitLab]: 'GitLab',
  [WebhookProvider.Generic]: 'Generic / CI',
};

const authSchemeLabels: Record<WebhookAuthScheme, string> = {
  [WebhookAuthScheme.GitHubHmacSha256]: 'GitHub HMAC SHA-256',
  [WebhookAuthScheme.GitLabSignedToken]: 'GitLab signed webhook',
  [WebhookAuthScheme.GitLabLegacyToken]: 'GitLab token',
  [WebhookAuthScheme.BearerToken]: 'Shared secret (Bearer header)',
};

const authSchemesByProvider: Record<WebhookProvider, WebhookAuthScheme[]> = {
  [WebhookProvider.GitHub]: [WebhookAuthScheme.GitHubHmacSha256],
  [WebhookProvider.GitLab]: [WebhookAuthScheme.GitLabSignedToken, WebhookAuthScheme.GitLabLegacyToken],
  [WebhookProvider.Generic]: [WebhookAuthScheme.BearerToken],
};

const defaultAuthScheme = (provider: WebhookProvider) => authSchemesByProvider[provider][0];
const fieldDescriptionClassName = 'text-sm text-muted-foreground';

function WebhookSubField({
  label,
  description,
  children,
  className,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
  className?: string | null;
}) {
  return (
    <div className={cn('grid gap-2', className)}>
      <div>
        <span className="text-sm font-medium">{label}</span>
        <p className={fieldDescriptionClassName}>{description}</p>
      </div>
      {children}
    </div>
  );
}

const generateSecret = () => {
  const bytes = new Uint8Array(32);
  globalThis.crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
};

const normalizeValue = (
  value: WebhookConfigValue | null | undefined,
  defaultBranch: string | null | undefined,
): NormalizedWebhookConfig => {
  const provider = value?.provider ?? WebhookProvider.GitHub;

  return {
    enabled: value?.enabled ?? false,
    provider,
    authScheme: value?.authScheme ?? defaultAuthScheme(provider),
    secret: value?.secret ?? '',
    branchFilter: value?.branchFilter ?? defaultBranch ?? '',
  };
};

export function WebhookConfigField({
  resourceType,
  resourceId,
  execution,
  value,
  defaultBranch,
  showBranchFilter = true,
  disabled,
  enableDisabled,
  onChange,
}: WebhookConfigFieldProps) {
  const [localProvider, setLocalProvider] = useState<WebhookProvider>(value?.provider ?? WebhookProvider.GitHub);
  const [localAuthScheme, setLocalAuthScheme] = useState<WebhookAuthScheme>(
    value?.authScheme ?? defaultAuthScheme(value?.provider ?? WebhookProvider.GitHub),
  );
  const normalized = normalizeValue(
    {
      provider: value?.provider ?? localProvider,
      authScheme: value?.authScheme ?? localAuthScheme,
      ...value,
    },
    defaultBranch,
  );
  const { enabled, provider, authScheme, branchFilter, secret } = normalized;
  const authType =
    provider === WebhookProvider.GitLab ? 'gitlab' : provider === WebhookProvider.Generic ? 'generic' : 'github';
  const listenerUrl = `${window.location.origin}/listener/${authType}/${resourceType}/${resourceId ?? '{resource-id}'}/${execution}`;
  const canCopyListenerUrl = !!resourceId;
  const [copiedUrl, copyUrl] = useCopyToClipboard(3000);

  const patch = (partial: Partial<WebhookCommonConfig>) => {
    onChange({
      ...(value ?? {}),
      ...partial,
    });
  };

  const handleProviderChange = (nextProvider: WebhookProvider) => {
    setLocalProvider(nextProvider);
    setLocalAuthScheme(defaultAuthScheme(nextProvider));
    patch({
      provider: nextProvider,
      authScheme: defaultAuthScheme(nextProvider),
      secret:
        nextProvider === WebhookProvider.Generic && !normalized.secret?.trim()
          ? generateSecret()
          : normalized.secret || null,
    });
  };

  return (
    <div className="flex flex-col gap-4">
      <FieldSwitch
        id={`${resourceType}-${execution}-webhook-enabled`}
        checked={enabled ?? false}
        disabled={disabled || (enableDisabled && !enabled)}
        onChange={(checked) => patch({ enabled: checked })}
      />

      {enabled && (
        <div className="border-t pt-3 flex flex-col gap-4">
          <WebhookSubField
            label="Provider"
            description="Select the provider format, or use Generic / CI for an authenticated resource-specific trigger.">
            <FieldSelect
              value={provider}
              onChange={handleProviderChange}
              disabled={disabled || enableDisabled}
              options={Object.values(WebhookProvider).map((item) => ({
                value: item,
                label: providerLabels[item],
              }))}
            />
          </WebhookSubField>

          <WebhookSubField
            label="Authentication"
            description={
              provider === WebhookProvider.Generic
                ? 'Generic webhooks send the configured shared secret in the Authorization: Bearer header.'
                : 'Choose how Citadel validates requests before running the webhook action.'
            }
            className={'pb-3'}>
            <FieldSelect
              value={authScheme}
              onChange={(next) => {
                setLocalAuthScheme(next);
                patch({ authScheme: next });
              }}
              disabled={disabled || enableDisabled}
              options={authSchemesByProvider[provider].map((item) => ({
                value: item,
                label: authSchemeLabels[item],
              }))}
            />
          </WebhookSubField>

          {showBranchFilter && (
            <WebhookSubField
              label="Branch"
              description="Only push events for this branch trigger the action. Leave empty to accept the resource default branch."
              className={'border-t pt-3 pb-2'}>
              <FieldInput
                value={branchFilter ?? undefined}
                placeholder="eg: main"
                disabled={disabled || enableDisabled}
                onChange={(next) => patch({ branchFilter: next || null })}
              />
            </WebhookSubField>
          )}

          <WebhookSubField
            label="Secret"
            description={
              provider === WebhookProvider.Generic
                ? 'Required shared secret for the CI system or external caller. This is not a Citadel user access token.'
                : 'Optional shared secret configured in the Git provider. Leave empty to accept unsigned webhook deliveries.'
            }
            className={'pt-3 border-t'}>
            <div className="flex max-w-140 flex-col gap-2 sm:flex-row">
              <FieldInput
                className="font-mono text-xs"
                value={secret ?? undefined}
                disabled={disabled || enableDisabled}
                onChange={(next) => patch({ secret: next || null })}
              />
              <Button
                type="button"
                variant="outline"
                className="w-fit gap-2"
                disabled={disabled || enableDisabled}
                onClick={() => patch({ secret: generateSecret() })}>
                <KeyRound className="size-3.5" />
                Generate
              </Button>
            </div>
          </WebhookSubField>

          <WebhookSubField
            label="Listener URL"
            description="Copy this URL into the provider or CI webhook settings. The final resource ID is available after creation."
            className={'pt-3 border-t'}>
            <div className="flex max-w-180 flex-col gap-2 sm:flex-row">
              <FieldInput
                className="max-w-full font-mono text-xs"
                value={listenerUrl}
                readOnly
                disabled={false}
                onChange={() => undefined}
              />
              <Button
                type="button"
                variant="outline"
                className="w-fit gap-2"
                disabled={!canCopyListenerUrl}
                onClick={() => copyUrl(listenerUrl)}>
                {copiedUrl ? <CheckCheck className="size-3.5 text-green-500" /> : <Clipboard className="size-3.5" />}
                Copy
              </Button>
            </div>
          </WebhookSubField>
        </div>
      )}
    </div>
  );
}
