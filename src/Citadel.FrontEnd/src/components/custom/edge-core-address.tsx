import { Input } from '@/components/ui/input';
import { ReactNode, useId, useState } from 'react';

export function EdgeCoreAddress({
  coreUrl,
  dockerCommand,
  children,
}: {
  coreUrl: string;
  dockerCommand: string;
  children: (command: string) => ReactNode;
}) {
  const inputId = useId();
  const [addressOverride, setAddressOverride] = useState<string>();
  const configured = parseAddress(coreUrl);
  const address = addressOverride ?? (configured && !isLocalAddress(configured.hostname) ? coreUrl : '');
  const parsed = parseAddress(address.trim());
  const assignment = shellQuote(`CITADEL_CORE_URL=${coreUrl}`);
  const canUpdateCommand = dockerCommand.includes(assignment);
  const command =
    parsed && canUpdateCommand
      ? dockerCommand.replace(assignment, () => shellQuote(`CITADEL_CORE_URL=${parsed.origin}`))
      : '';

  return (
    <div className="grid gap-3">
      <div className="grid gap-2 text-sm">
        <label htmlFor={inputId} className="font-medium">
          Core address
        </label>
        <Input
          id={inputId}
          type="url"
          value={address}
          className="sm:max-w-100"
          placeholder={`${configured?.protocol ?? 'https:'}//core-host${configured?.port ? `:${configured.port}` : ''}`}
          onChange={(event) => setAddressOverride(event.target.value)}
          aria-describedby={`${inputId}-help`}
          aria-invalid={Boolean(address && !parsed)}
        />
        <p id={`${inputId}-help`} className="text-muted-foreground">
          Enter the Core machine’s IP address or hostname and Edge port, reachable from the Agent machine. This updates
          the installation command below; it does not change Core’s configuration. Use host.docker.internal only when
          Core runs on the Agent’s Docker host.
        </p>
        {address && !parsed && (
          <p role="alert" className="text-destructive">
            Enter an HTTP or HTTPS address with no credentials, path, query or fragment.
          </p>
        )}
        {parsed && !canUpdateCommand && (
          <p role="alert" className="text-destructive">
            Generate a new enrollment command to configure its Core address.
          </p>
        )}
      </div>
      {command ? (
        children(command)
      ) : (
        <p className="text-sm text-muted-foreground">Enter Core’s address to display the Docker command.</p>
      )}
    </div>
  );
}

function parseAddress(value: string): URL | undefined {
  try {
    const url = new URL(value);
    if (
      ['http:', 'https:'].includes(url.protocol) &&
      !url.username &&
      !url.password &&
      url.pathname === '/' &&
      !url.search &&
      !url.hash
    )
      return url;
  } catch {
    // The address may be incomplete while the user types.
  }
}

function isLocalAddress(hostname: string): boolean {
  return (
    ['host.docker.internal', 'gateway.docker.internal', 'localhost', '[::1]', '[::]', '0.0.0.0'].includes(hostname) ||
    hostname.endsWith('.localhost') ||
    hostname.startsWith('127.')
  );
}

function shellQuote(value: string): string {
  return `'${value.replaceAll("'", "'\"'\"'")}'`;
}
