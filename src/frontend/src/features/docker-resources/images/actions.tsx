import { ImageView } from '@/api/generated/api.types';
import { useAppContext } from '@/lib/context/app-context';
import { formatId } from '@/lib/utils';
import { SearchCode } from 'lucide-react';
import { useNavigate } from 'react-router';
import { createActionsBuilder } from '@/components/custom/actions-builder';
import { ImageReferenceButton, ImageReferenceDropdown } from './reference-actions';
import { DeleteImageButton, DeleteImageDropdown } from './delete-image-dialog';

const { dropdown, group } = createActionsBuilder<ImageView>()
  .addAction({
    key: 'inspect',
    type: 'command',
    icon: SearchCode,
    requiredCapabilities: ['canInspect'],
    useHandler: ({ resources }) => {
      const navigate = useNavigate();
      const { currentPlatform } = useAppContext();
      const selected = Array.isArray(resources) ? resources[0] : resources;
      let canExecute = selected?.capabilities?.canInspect === true;
      if (Array.isArray(resources)) {
        canExecute &&= resources.length === 1;
      }
      return {
        canExecute,
        isPending: false,
        run: () => {
          if (!canExecute || !selected) return;
          const nodeQuery = selected.dockerNodeId ? `?dockerNodeId=${encodeURIComponent(selected.dockerNodeId)}` : '';
          navigate(`/platforms/${currentPlatform?.id}/images/${formatId(selected.dockerImageId)}/${nodeQuery}`);
        },
      };
    },
  })
  .build();

export const ImageDropdownActions: typeof dropdown = {
  ...dropdown,
  check: (props) => <ImageReferenceDropdown {...props} kind="check" />,
  pull: (props) => <ImageReferenceDropdown {...props} kind="pull" />,
  delete: DeleteImageDropdown,
};
export const ImageGroupActions: typeof group = {
  ...group,
  check: ({ resources }) => <ImageReferenceButton resources={resources} kind="check" />,
  pull: ({ resources }) => <ImageReferenceButton resources={resources} kind="pull" />,
  delete: ({ resources }: { resources: ImageView[] }) => <DeleteImageButton resources={resources} />,
};
