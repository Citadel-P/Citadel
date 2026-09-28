import { DockerVolumeResultView } from '@/api/generated/api.types';
import { DetailFacts } from '@/components/custom/resource-detail';
import { byteTransform } from '@/lib/bytes.helper';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

export const VolumeInfoTable = ({ volume }: { volume: DockerVolumeResultView | undefined }) => {
  const formatDateTime = useProfileDateTimeFormatter();
  if (!volume) return null;
  const size = volume.usageData?.size;
  return (
    <DetailFacts
      resource={volume}
      items={[
        { label: 'Driver', value: volume.driver },
        { label: 'Scope', value: volume.scope },
        {
          label: 'Size',
          value:
            size != null && Number.isFinite(Number(size)) && Number(size) >= 0
              ? byteTransform(size, 2)
              : 'Not available',
        },
        { label: 'Created', value: <TimestampCell value={volume.createdAt} formatDateTime={formatDateTime} /> },
        { label: 'Mount point', value: volume.mountpoint || 'Not available' },
        { label: 'Attached containers', value: volume.containers?.length ?? 0 },
      ]}
    />
  );
};
