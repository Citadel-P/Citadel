import { CheckCircle2, Clock3 } from 'lucide-react';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { cn } from '@/lib/utils';

export interface BuildImageProvenance {
  resolvedImageReference?: null | string;
  resolvedDigest?: null | string;
  resolvedBuildRunId?: null | string;
  appliedImageReference?: null | string;
  appliedDigest?: null | string;
  appliedBuildRunId?: null | string;
  appliedAt?: null | string;
}

export const BuildImageProvenanceStatus = ({
  value,
  latestImageReference,
  latestDigest,
  latestBuildRunId,
}: {
  value?: BuildImageProvenance | null;
  latestImageReference?: string | null;
  latestDigest?: string | null;
  latestBuildRunId?: string | null;
}) => {
  const formatDateTime = useProfileDateTimeFormatter();
  const desiredReference = value?.resolvedImageReference;
  const appliedReference = value?.appliedImageReference;
  const isApplied = sameArtifact(value);
  const hasDesired = !!desiredReference;
  const hasApplied = !!appliedReference;
  const hasLatest = !!latestImageReference;

  return (
    <div className="grid gap-2 border-l-2 border-border pl-3 text-xs">
      <ProvenanceRow label="Latest" value={latestImageReference} detail={latestDigest} />
      <ProvenanceRow
        label="Desired"
        value={desiredReference}
        detail={value?.resolvedDigest}
        emptyValue={latestBuildRunId ? 'Will resolve on apply' : 'Not resolved'}
      />
      <ProvenanceRow label="Applied" value={appliedReference} detail={value?.appliedDigest} emptyValue="Not applied" />
      <div
        className={cn(
          'flex items-center gap-1.5',
          isApplied
            ? 'text-emerald-600'
            : hasDesired || hasApplied
              ? 'text-amber-600'
              : hasLatest
                ? 'text-blue-600'
                : 'text-muted-foreground',
        )}>
        {isApplied ? <CheckCircle2 className="size-3.5" /> : <Clock3 className="size-3.5" />}
        <span>
          {isApplied
            ? `Applied${value?.appliedAt ? ` ${formatDateTime(value.appliedAt)}` : ''}`
            : hasDesired
              ? 'Apply required'
              : hasLatest
                ? 'Ready to apply'
                : 'Waiting for a successful build'}
        </span>
      </div>
    </div>
  );
};

const ProvenanceRow = ({
  label,
  value,
  detail,
  emptyValue = 'Unavailable',
}: {
  label: string;
  value?: string | null;
  detail?: string | null;
  emptyValue?: string;
}) => (
  <div className="grid min-w-0 grid-cols-[3.75rem_minmax(0,1fr)] gap-2">
    <span className="text-muted-foreground">{label}</span>
    {value ? (
      <span className="min-w-0">
        <span className="block truncate font-mono" title={value}>
          {value}
        </span>
        {detail && detail !== value ? (
          <span className="block truncate font-mono text-muted-foreground" title={detail}>
            {detail}
          </span>
        ) : null}
      </span>
    ) : (
      <span className="text-muted-foreground">{emptyValue}</span>
    )}
  </div>
);

const matches = (left?: string | null, right?: string | null) =>
  !!left && !!right && left.toLowerCase() === right.toLowerCase();

const sameArtifact = (value?: BuildImageProvenance | null) => {
  if (value?.resolvedDigest && value.appliedDigest) return matches(value.resolvedDigest, value.appliedDigest);
  if (value?.resolvedBuildRunId && value.appliedBuildRunId)
    return matches(value.resolvedBuildRunId, value.appliedBuildRunId);
  return matches(value?.resolvedImageReference, value?.appliedImageReference);
};
