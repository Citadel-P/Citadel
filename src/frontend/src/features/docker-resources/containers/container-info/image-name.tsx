import type { ImageView } from '@/api/generated/api.types';
import { Link } from 'react-router';
import { truncate } from '@/lib/truncate';

export const ImageName = ({ image }: { image: ImageView | undefined }) => {
  if (image === undefined || image?.name === undefined) return <div className="text-muted-foreground">{'<none>'}</div>;
  return (
    <Link
      to={`/platforms/${image.platformId}/images/${image?.dockerImageId?.slice(0, 24)}`}
      title={image?.name}
      className="table-link">
      {truncate(image?.name?.length > 0 ? image?.name : '<none>', 24)}
    </Link>
  );
};
