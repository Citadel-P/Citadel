import { ImageInspectionView } from '@/api/generated/api.types';
import { DetailFacts } from '@/components/custom/resource-detail';
import { RegistryDisplay } from '@/components/custom/registry-display';
import { byteTransform } from '@/lib/bytes.helper';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';

export const ImageInfoTable = ({ image }: { image: ImageInspectionView | undefined }) => {
  const formatDateTime = useProfileDateTimeFormatter();
  if (!image) return null;
  return (
    <DetailFacts
      resource={image}
      items={[
        { label: 'Operating system', value: image.os },
        { label: 'Architecture', value: image.architecture },
        { label: 'Size', value: byteTransform(image.size, 2) },
        { label: 'Registry', value: image.registry ? <RegistryDisplay registry={image.registry} /> : 'Not linked' },
        { label: 'Created', value: <TimestampCell value={image.created} formatDateTime={formatDateTime} /> },
        { label: 'Exposed ports', value: image.exposedPorts?.join(', ') || 'None' },
      ]}
    />
  );
};
