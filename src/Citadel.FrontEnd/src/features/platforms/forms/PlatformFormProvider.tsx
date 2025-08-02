import { PlatformView, PlatformType } from '@/api/_generated';
import { createContext, JSX, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { useNavigate, useParams } from 'react-router';
import { useGETPlatform } from './../hooks/useGETPlatform';
import { usePOSTPlatform } from './hooks/usePOSTPlatform';
import { usePATCHPlatform } from './hooks/usePATCHPlatform';
import { useRequiredContext } from '@/hooks/useRequiredContext';
import AddDockerPlatform from '../addDockerPlatform/AddDockerPltaform';

interface IPlatformProvider {
  id: 'docker' | 'swarm' | 'k8s';
  name: string;
  description: string;
  configuration: JSX.Element;
  disabled: boolean;
}

interface IContext {
  mode: FormMode;
  formTitle: string;
  isLoading: boolean;
  isLoadingForm: boolean;
  saveButtonTitle: string;
  validationErrors: string | undefined | null;
  platform: PlatformView | undefined;
  providers: IPlatformProvider[];
  currentProvider: string;
  setCurrentProvider: (value: string) => void;
}
interface IProps {
  children?: React.ReactNode;
}

const PlatformFormContext = createContext<IContext | undefined>(undefined);

const PlatformFormProvider: React.FC<IProps> = ({ children }) => {
  const defaultProviders: IPlatformProvider[] = [
    {
      id: 'docker',
      name: 'Docker',
      description: 'Docker standalone',
      configuration: <AddDockerPlatform />,
      disabled: false,
    },
    {
      id: 'swarm',
      name: 'Docker Swarm',
      description: 'Manage a cluster of Docker daemons',
      configuration: <></>,
      disabled: true,
    },
    {
      id: 'k8s',
      name: 'Kubernetes',
      description: 'K8S container orchestration platform',
      configuration: <></>,
      disabled: true,
    },
  ] as const;

  const navigate = useNavigate();
  const { platformId } = useParams();
  const mode: FormMode = platformId ? 'edit' : 'add';
  const { data, isLoading } = useGETPlatform(platformId);
  const {
    mutate: _requestCreate,
    validationErrors: createErrors,
    isSuccess: createIsSuccess,
    isPending: createIsPending,
    data: createData,
  } = usePOSTPlatform();
  const {
    mutate: _requestPatch,
    validationErrors: patchErrors,
    isSuccess: patchIsSuccess,
    isPending: patchIsPending,
    data: patchData,
  } = usePATCHPlatform();

  const [currentProvider, setCurrentProvider] = useState<string>(PlatformType.Docker);
  const [platform, setPlatform] = useState<PlatformView | undefined>(undefined);
  let providers = [...defaultProviders];
  let formTitle = 'Create platform';
  let saveButtonTitle = 'Add platform';

  if (mode === 'edit' && data?.data) {
    formTitle = 'Update platform';
    saveButtonTitle = 'Save';
    providers = providers.map((s) => {
      s.disabled = s.id !== currentProvider;
      return s;
    });
  }
  useEffect(() => {
    if (mode === 'edit' && data?.data) {
      setCurrentProvider(data?.data.type);
      setPlatform(data?.data);
    }
  }, [data, mode]);

  useEffect(() => {
    if (createIsSuccess && createData?.data) {
      toast.success(`The ${createData?.data.name} platform has been added`);
      setPlatform(createData?.data);
      navigate('/platforms');
    }
  }, [createIsSuccess, createData, navigate]);

  useEffect(() => {
    if (patchIsSuccess && patchData?.data) {
      toast.success(`The ${patchData?.data.name} platform has been updated successfully`);
      setPlatform(patchData?.data);
      navigate('/platforms');
    }
  }, [patchIsSuccess, patchData, navigate]);

  // function onPostForm(data: PlatformInput | Partial<PlatformInput>) {
  //   if (mode === 'add') {
  //     requestCreate(data as PlatformInput);
  //   } else {
  //     requestPatch(payload as PlatformInput);
  //   }
  // }

  return (
    <PlatformFormContext.Provider
      value={{
        mode,
        isLoading,
        isLoadingForm: createIsPending || patchIsPending,
        formTitle,
        saveButtonTitle,
        platform,
        validationErrors: createErrors || patchErrors,
        providers,
        currentProvider,
        setCurrentProvider,
      }}>
      {children}
    </PlatformFormContext.Provider>
  );
};

type FormMode = 'edit' | 'add';

export default PlatformFormProvider;
export const usePlatformFormContext = () => useRequiredContext(PlatformFormContext);
