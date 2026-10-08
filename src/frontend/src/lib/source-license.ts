const DEFAULT_REPOSITORY_URL = 'https://github.com/Citadel-P/Citadel';
const FULL_REVISION = /^[0-9a-f]{40}$/i;
const EMBEDDED_REVISION = /(?:^|[.+-])sha\.([0-9a-f]{40})(?=$|[.+-])/i;

/**
 * Resolve links to the exact revision of an official build when available.
 * Modified builds can set their own source-repository URL for provenance.
 */
export function sourceAndLicenseLinks({
  informationalVersion,
  repositoryUrl = import.meta.env.VITE_CITADEL_SOURCE_REPOSITORY_URL || DEFAULT_REPOSITORY_URL,
  buildRevision = import.meta.env.VITE_CITADEL_BUILD_SHA || '',
}: {
  informationalVersion?: string;
  repositoryUrl?: string;
  buildRevision?: string;
} = {}) {
  const fromVersion = informationalVersion?.match(EMBEDDED_REVISION)?.[1];
  const revision = fromVersion ?? (FULL_REVISION.test(buildRevision) ? buildRevision : undefined);
  const repository = repositoryUrl.replace(/\/+$/, '');
  return {
    sourceUrl: revision ? repository + '/tree/' + revision.toLowerCase() : repository,
    licenseUrl: repository + '/blob/' + (revision ? revision.toLowerCase() : 'main') + '/LICENSE',
  };
}
