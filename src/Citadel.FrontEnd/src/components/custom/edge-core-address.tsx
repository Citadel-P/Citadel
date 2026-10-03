import type { ReactNode } from 'react';

export function EdgeCoreAddress({
  coreUrl,
  dockerCommand,
  children,
}: {
  coreUrl: string;
  dockerCommand: string;
  children: (command: string) => ReactNode;
}) {
  const configured = parseAddress(coreUrl);
  const placeholder = `${configured?.protocol ?? 'http:'}//core-ip-or-hostname${configured?.port ? `:${configured.port}` : ''}`;
  const address = configured && !isLocalAddress(configured.hostname) ? configured.origin : placeholder;
  const assignment = shellQuote(`CITADEL_CORE_URL=${coreUrl}`);
  const canUpdateCommand = dockerCommand.includes(assignment);
  const command = dockerCommand.replace(assignment, () => shellQuote(`CITADEL_CORE_URL=${address}`));

  return (
    <div className="grid gap-3">
      {canUpdateCommand ? (
        <>
          <p className="text-sm text-muted-foreground">
            {address === placeholder
              ? 'Replace core-ip-or-hostname with the Core IP address or hostname reachable from the Agent machine. Keep the Edge port.'
              : 'Use a Core address reachable from the Agent machine.'}
          </p>
          {children(command)}
        </>
      ) : (
        <p role="alert" className="text-sm text-destructive">
          Generate a new enrollment command to configure its Core address.
        </p>
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
    // Fall back to a placeholder when the configured address is invalid.
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
