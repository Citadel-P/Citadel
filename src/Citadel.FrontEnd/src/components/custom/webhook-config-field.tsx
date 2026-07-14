import {
  AutomationWebhookConfig,
  BackupWebhookConfig,
  RepoWebhookConfig,
  StackWebhookConfig,
  WebhookAuthScheme,
  WebhookProvider,
} from '@/api/generated/api.types';
import { FieldInput, FieldSelect, FieldSwitch } from '@/components/custom/form-builder';
import { Button } from '@/components/ui/button';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { cn } from '@/lib/utils';
import { CheckCheck, Clipboard, KeyRound } from 'lucide-react';
import { useState } from 'react';

type WebhookConfigValue = RepoWebhookConfig | StackWebhookConfig | AutomationWebhookConfig | BackupWebhookConfig;
type WebhookCommonConfig = Pick<RepoWebhookConfig, 'enabled' | 'provider' | 'authScheme' | 'secret' | 'branchFilter'>;
type NormalizedWebhookConfig = Required<Pick<WebhookCommonConfig, 'enabled' | 'provider' | 'authScheme'>> &
  Pick<WebhookCommonConfig, 'secret' | 'branchFilter'>;

type WebhookConfigFieldProps = {
  resourceType: 'repo' | 'stack' | 'automation-action' | 'backup-policy';
  resourceId?: string;
  execution: 'pull' | 'deploy' | 'run';
  value?: WebhookConfigValue | null;
  defaultBranch?: string | null;
  showBranchFilter?: boolean;
  disabled?: boolean;
  onChange: (value: WebhookConfigValue) => void;
};

const providerLabels: Record<WebhookProvider, string> = {
  [WebhookProvider.GitHub]: 'GitHub',
  [WebhookProvider.GitLab]: 'GitLab',
};

const authSchemeLabels: Record<WebhookAuthScheme, string> = {
  [WebhookAuthScheme.GitHubHmacSha256]: 'GitHub HMAC SHA-256',
  [WebhookAuthScheme.GitLabSignedToken]: 'GitLab signed webhook',
  [WebhookAuthScheme.GitLabLegacyToken]: 'GitLab token',
};

const authSchemesByProvider: Record<WebhookProvider, WebhookAuthScheme[]> = {
  [WebhookProvider.GitHub]: [WebhookAuthScheme.GitHubHmacSha256],
  [WebhookProvider.GitLab]: [WebhookAuthScheme.GitLabSignedToken, WebhookAuthScheme.GitLabLegacyToken],
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
  globalThis.crypto?.getRandomValues(bytes);

  if (bytes.some((byte) => byte !== 0)) {
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  }

  return Array.from({ length: 4 }, () => Math.random().toString(16).slice(2).padEnd(16, '0'))
    .join('')
    .slice(0, 64);
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
  const authType = provider === WebhookProvider.GitLab ? 'gitlab' : 'github';
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
    });
  };

  return (
    <div className="flex flex-col gap-4">
      <FieldSwitch
        id={`${resourceType}-${execution}-webhook-enabled`}
        checked={enabled ?? false}
        disabled={disabled}
        onChange={(checked) => patch({ enabled: checked })}
      />

      {enabled && (
        <div className="border-t pt-3 flex flex-col gap-4">
          <WebhookSubField
            label="Provider"
            description="Select the Git provider signature format used by the incoming webhook.">
            <FieldSelect
              value={provider}
              onChange={handleProviderChange}
              disabled={disabled}
              options={Object.values(WebhookProvider).map((item) => ({
                value: item,
                label: providerLabels[item],
              }))}
            />
          </WebhookSubField>

          <WebhookSubField
            label="Authentication"
            description="Choose how Citadel validates requests before running the webhook action."
            className={'pb-3'}>
            <FieldSelect
              value={authScheme}
              onChange={(next) => {
                setLocalAuthScheme(next);
                patch({ authScheme: next });
              }}
              disabled={disabled}
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
                value={branchFilter}
                placeholder="eg: main"
                disabled={disabled}
                onChange={(next) => patch({ branchFilter: next || null })}
              />
            </WebhookSubField>
          )}

          <WebhookSubField
            label="Secret"
            description="Optional shared secret configured in the Git provider. Leave empty to accept unsigned webhook deliveries."
            className={'pt-3 border-t'}>
            <div className="flex max-w-140 flex-col gap-2 sm:flex-row">
              <FieldInput
                className="font-mono text-xs"
                value={secret}
                disabled={disabled}
                onChange={(next) => patch({ secret: next || null })}
              />
              <Button
                type="button"
                variant="outline"
                className="w-fit gap-2"
                disabled={disabled}
                onClick={() => patch({ secret: generateSecret() })}>
                <KeyRound className="size-3.5" />
                Generate
              </Button>
            </div>
          </WebhookSubField>

          <WebhookSubField
            label="Listener URL"
            description="Copy this URL into the Git provider webhook settings. The final resource ID is available after creation."
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
