import { useState } from 'react';
import { ChevronDown, Terminal } from 'lucide-react';
import { useRead } from '@/lib/hooks';
import { hasCapability } from '@/lib/resource-capabilities';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { ContainerDetailsView } from '../hooks/useContainerInfoGroup';
import { ContainerHealthSummary, currentHealth, HealthLabel, healthTime, useHealthClock } from '../health';
import { ContainerHealthHistory } from './container-health-history';
import { useContainerHealthHistory } from '../hooks/useContainerHealthHistory';
import { inactiveHealthCheckMessage } from '@/components/custom/health-status';

const object = (value: unknown): Record<string, unknown> =>
  value != null && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
const duration = (value: unknown) =>
  typeof value !== 'number' ? 'Not reported' : value > 0 ? `${value / 1_000_000_000} s` : 'Docker default';

export function ContainerHealth({ resource }: { resource: ContainerDetailsView }) {
  const now = useHealthClock();
  const status = currentHealth(resource, now);
  const inactiveMessage = inactiveHealthCheckMessage(status);
  const canInspect = hasCapability(resource, 'canInspect');
  const [loadInspection, setLoadInspection] = useState(false);
  const inspection = useRead(
    'inspectContainer',
    { id: resource.resourceId },
    {
      enabled: canInspect && loadInspection && !inactiveMessage,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
    },
  );
  const history = useContainerHealthHistory(resource.resourceId, !inactiveMessage);
  const document = object(inspection.data?.data);
  const check = object(object(document.config).healthcheck);
  const health = object(object(document.state).health);
  const logs = Array.isArray(health.log) ? health.log.map(object).reverse() : [];
  const command = Array.isArray(check.test) ? check.test.filter((s): s is string => typeof s === 'string') : [];
  return (
    <div className="space-y-6">
      <section aria-label="Current health" className="rounded-lg border bg-muted/20 p-4">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-sm font-semibold">Current health</h2>
          <HealthLabel status={status} />
        </div>
        <ContainerHealthSummary container={resource} layout="metrics" />
      </section>

      <ContainerHealthHistory
        entries={history.entries ?? []}
        now={now}
        isConnected={history.isConnected}
        isLoading={history.isLoading}
        disabledReason={inactiveMessage}
      />

      {!inactiveMessage && (
        <>
          <Collapsible
            onOpenChange={(open) => {
              if (open) setLoadInspection(true);
            }}
            className="rounded-lg border">
            <h2 id="health-checks-title" className="text-sm font-semibold">
              <CollapsibleTrigger asChild>
                <Button
                  variant="ghost"
                  aria-label="Probe details"
                  className="h-auto w-full justify-start gap-3 rounded-lg p-4 text-left [&[data-state=open]>svg:last-child]:rotate-180">
                  <Terminal className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
                  <span className="space-y-1">
                    <span className="block text-sm font-medium">Probe details</span>
                    <span className="block whitespace-normal text-xs font-normal text-muted-foreground">
                      Recent output and effective check settings
                    </span>
                  </span>
                  <ChevronDown className="ml-auto size-4 transition-transform" aria-hidden="true" />
                </Button>
              </CollapsibleTrigger>
            </h2>
            <CollapsibleContent role="region" aria-labelledby="health-checks-title" className="px-4 pb-4">
              {!canInspect ? (
                <p className="text-sm text-muted-foreground">
                  Inspect permission is required to view the command and probe output.
                </p>
              ) : !loadInspection || inspection.isLoading ? (
                <p role="status" className="text-sm text-muted-foreground">
                  Loading check details…
                </p>
              ) : inspection.isError ? (
                <p role="alert" className="text-sm">
                  Current inspection is unavailable. Saved health history is shown above.
                </p>
              ) : document.healthMetadataObserved !== true ? (
                <p className="text-sm text-muted-foreground">
                  This runtime did not provide health-check metadata. Update the Agent to expose these details.
                </p>
              ) : (
                <div className="space-y-4">
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    Docker’s latest available results, newest first. These may predate the current container state.
                  </p>
                  {logs.length === 0 && (
                    <p className="py-4 text-center text-sm text-muted-foreground">No recent probe results available.</p>
                  )}
                  <div className="space-y-2">
                    {logs.map((log, index) => (
                      <details key={index} className="group rounded-lg border">
                        <summary className="flex cursor-pointer list-none flex-wrap items-center gap-2 rounded-lg p-3 text-xs focus-visible:outline-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
                          <Badge
                            variant="outline"
                            className={
                              typeof log.exitCode !== 'number'
                                ? 'text-muted-foreground'
                                : log.exitCode === 0
                                  ? 'text-emerald-600 dark:text-emerald-400'
                                  : 'text-destructive'
                            }>
                            {typeof log.exitCode === 'number' ? `Exit ${log.exitCode}` : 'Exit unknown'}
                          </Badge>
                          <span className="text-muted-foreground">
                            {healthTime(typeof log.end === 'string' ? log.end : null)}
                          </span>
                          <span className="ml-auto inline-flex items-center gap-1">
                            Output{' '}
                            <ChevronDown
                              className="size-3.5 transition-transform group-open:rotate-180"
                              aria-hidden="true"
                            />
                          </span>
                        </summary>
                        <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-all border-t bg-muted/40 p-3 font-mono text-xs leading-relaxed">
                          {typeof log.output === 'string' ? log.output : 'No output retained.'}
                        </pre>
                      </details>
                    ))}
                  </div>
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    Up to five results, with output limited to 4,096 characters and known sensitive environment values
                    redacted. Probe output is not saved in health history.
                  </p>

                  <details className="group rounded-lg border">
                    <summary className="flex cursor-pointer list-none items-center gap-2 rounded-lg p-3 text-sm font-medium focus-visible:outline-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
                      <Terminal className="size-4 text-muted-foreground" aria-hidden="true" /> Health check
                      configuration
                      <ChevronDown
                        className="ml-auto size-4 transition-transform group-open:rotate-180"
                        aria-hidden="true"
                      />
                    </summary>
                    <div className="space-y-4 border-t p-3">
                      <p className="text-xs text-muted-foreground">
                        Effective settings on this container. Unapplied workload configuration may differ.
                      </p>
                      <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all rounded-md bg-muted p-3 text-xs">
                        {command[0] === 'NONE'
                          ? 'Health check disabled.'
                          : command.length
                            ? JSON.stringify(command)
                            : 'No health check configured.'}
                      </pre>
                      <dl className="grid gap-4 text-xs sm:grid-cols-3">
                        {[
                          ['Interval', duration(check.interval)],
                          ['Timeout', duration(check.timeout)],
                          ['Startup period', duration(check.startPeriod)],
                          ['Startup interval', duration(check.startInterval)],
                          [
                            'Retries',
                            typeof check.retries !== 'number'
                              ? 'Not reported'
                              : check.retries > 0
                                ? check.retries
                                : 'Docker default',
                          ],
                        ].map(([label, value]) => (
                          <div key={label} className="space-y-1">
                            <dt className="text-muted-foreground">{label}</dt>
                            <dd className="font-medium">{value}</dd>
                          </div>
                        ))}
                      </dl>
                    </div>
                  </details>
                </div>
              )}
            </CollapsibleContent>
          </Collapsible>
          <p className="text-xs leading-relaxed text-muted-foreground">
            Probe details load on demand. Viewing them never runs a check or restarts the container.
          </p>
        </>
      )}
    </div>
  );
}
