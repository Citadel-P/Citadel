import { LicenseCapability } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';

export const LICENSE_ENTITLEMENTS_STALE_TIME = 60 * 60 * 1000;

export function useLicenseEntitlements() {
  const query = useRead('getLicenseEntitlements', undefined, {
    retry: false,
    staleTime: LICENSE_ENTITLEMENTS_STALE_TIME,
    refetchOnWindowFocus: true,
    meta: { suppressErrorToast: true },
  });

  const capabilities = query.data?.data?.capabilities ?? [];
  const hasCapability = (capability: LicenseCapability) =>
    capabilities.some((item) => item.capability === capability && item.enabled);

  return {
    ...query,
    entitlements: query.data?.data,
    hasCapability,
  };
}
