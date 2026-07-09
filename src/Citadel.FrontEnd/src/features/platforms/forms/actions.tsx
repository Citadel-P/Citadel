import { EdgeAgentEnrollmentView } from '@/api/generated/api.types';
import { Button } from '@/components/ui/button';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { cn } from '@/lib/utils';
import { CheckCheck, Clipboard, KeyRound, Loader2, Terminal } from 'lucide-react';
import { ComponentType, useMemo } from 'react';

const agentVersion = '1.0.0';

export const PlatformEnrollmentActions = ({
  platformName,
  enrollment,
  isPending,
  onRegenerate,
}: {
  platformName?: string;
  enrollment?: EdgeAgentEnrollmentView;
  isPending?: boolean;
  onRegenerate: () => void;
}) => {
  const command = useMemo(
    () => (enrollment ? buildDockerRunCommand(enrollment.instructions.environment) : ''),
    [enrollment],
  );

  if (!enrollment) {
    return (
      <div className="flex flex-col gap-3 rounded-md border border-border bg-muted/20 p-4">
        <div>
          <div className="text-sm font-semibold">Edge Agent platform created</div>
          <div className="text-xs text-muted-foreground">
            {platformName ?? 'Platform'} is ready for an enrollment token.
          </div>
        </div>
        <Button type="button" onClick={onRegenerate} disabled={isPending}>
          {isPending && <Loader2 className="size-4 animate-spin" />}
          Generate Enrollment Token
        </Button>
      </div>
    );
  }

  return (
    <div className="grid gap-4 rounded-md border border-border bg-muted/20 p-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="flex items-start gap-3">
          <KeyRound className="mt-0.5 size-4 text-primary" />
          <div className="grid gap-1">
            <div className="text-sm font-semibold">Enrollment token generated</div>
            <div className="text-xs text-muted-foreground">
              Expires {formatDate(enrollment.expiresAtUtc)}. The token is shown once and stored only as a hash.
            </div>
          </div>
        </div>
        <Button type="button" size="sm" variant="outline" onClick={onRegenerate} disabled={isPending}>
          {isPending && <Loader2 className="size-3.5 animate-spin" />}
          Generate New Token
        </Button>
      </div>

      <InstructionBlock icon={Terminal} title="Docker command" value={command} />

      <div className="grid gap-2">
        <div className="text-sm font-medium">Environment</div>
        <div className="grid gap-2">
          {Object.entries(enrollment.instructions.environment).map(([key, value]) => (
            <InstructionBlock key={key} title={key} value={value} compact />
          ))}
        </div>
      </div>
    </div>
  );
};

const InstructionBlock = ({
  icon: Icon,
  title,
  value,
  compact,
}: {
  icon?: ComponentType<{ className?: string }>;
  title: string;
  value: string;
  compact?: boolean;
}) => {
  const [copied, copy] = useCopyToClipboard(3000);
  const isCopied = copied === value;

  return (
    <div className="grid gap-1">
      <div className="flex items-center gap-2 text-sm font-medium">
        {Icon && <Icon className="size-4 text-muted-foreground" />}
        {title}
      </div>
      <div className="flex items-start gap-2 rounded-md border bg-background p-2">
        <pre
          className={cn(
            'min-w-0 flex-1 whitespace-pre-wrap break-all font-mono text-xs text-foreground',
            compact ? 'leading-5' : 'leading-6',
          )}>
          {value}
        </pre>
        <Button type="button" size="icon-xs" variant="ghost" onClick={() => copy(value)}>
          {isCopied ? <CheckCheck className="size-3 text-green-500" /> : <Clipboard className="size-3" />}
          <span className="sr-only">Copy</span>
        </Button>
      </div>
    </div>
  );
};

const formatDate = (value: unknown) => {
  if (!value) return '-';
  return new Date(value as string).toLocaleString();
};

const buildDockerRunCommand = (environment: Record<string, string>) => {
  const env = Object.entries(environment)
    .map(([key, value]) => `  -e ${key}="${escapeDockerValue(value)}"`)
    .join(' \\\n');

  return `docker run -d \\
  --name citadel-agent \\
  --restart=always \\
  -v /var/run/docker.sock:/var/run/docker.sock \\
  -v citadel_edge_agent_data:/app/data \\
${env} \\
  Citadel/agent:${agentVersion}`;
};

const escapeDockerValue = (value: string) => value.replace(/"/g, '\\"');
