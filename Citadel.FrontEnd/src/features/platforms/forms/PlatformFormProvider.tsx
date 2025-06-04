import { RegistryInput, PlatformView, RegistryType, RegistryView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { JSX, useEffect, useState } from 'react';
import { toast } from 'sonner';
import DockerHubConfiguration from './DockerHubConfiguration';
import GhcrConfiguration from './GhcrConfiguration';
import { useNavigate, useParams } from 'react-router';
import { useGetRegistry } from './hooks/useGetRegistry';
import { usePOSTRegistry } from './hooks/usePOSTRegistry';
import { usePATCHRegistry } from './hooks/usePATCHRegistry';
import { useGETPlatform } from '../hooks/useGETPlatform';

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
      configuration: <DockerHubConfiguration />,
      disabled: false,
    },
    {
      id: 'swarm',
      name: 'Docker Swarm',
      description: 'Manage a cluster of Docker daemons',
      configuration: <GhcrConfiguration />,
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
    mutate: requestCreate,
    validationErrors: createErrors,
    isSuccess: createIsSuccess,
    isPending: createIsPending,
    data: createData,
  } = usePOSTPlatform();
  const {
    mutate: requestPatch,
    validationErrors: patchErrors,
    isSuccess: patchIsSuccess,
    isPending: patchIsPending,
    data: patchData,
  } = usePATCHRegistry();
  const [currentProvider, setCurrentProvider] = useState<string>(RegistryType.DockerHub);
  const [registry, setRegistry] = useState<RegistryView | undefined>(undefined);
  let providers = [...defaultProviders];
  let formTitle = 'Create registry';
  let saveButtonTitle = 'Add registry';

  if (mode === 'edit' && data?.data) {
    formTitle = 'Update registry';
    saveButtonTitle = 'Save';
    providers = providers.map((s) => {
      s.disabled = s.id !== currentProvider;
      return s;
    });
  }
  useEffect(() => {
    if (mode === 'edit' && data?.data) {
      setCurrentProvider(data?.data.type);
      setRegistry(data?.data);
    }
  }, [data, mode]);

  useEffect(() => {
    if (createIsSuccess && createData?.data) {
      toast.success(`The ${createData?.data.name} registry has been added`);
      setRegistry(createData?.data);
      navigate('/registries');
    }
  }, [createIsSuccess, createData, navigate]);

  useEffect(() => {
    if (patchIsSuccess && patchData?.data) {
      toast.success(`The ${patchData?.data.name} registry has been updated successfully`);
      setRegistry(patchData?.data);
      navigate('/registries');
    }
  }, [patchIsSuccess, patchData, navigate]);

  function onPostForm(data: RegistryInput | Partial<RegistryInput>) {
    if (mode === 'add') {
      requestCreate(data as RegistryInput);
    } else {
      // for serialization
      const payload = {
        ...data,
        id: registry?.id,
        discriminator: registry?.type,
        configuration: { $type: registry?.configuration.$type, ...data.configuration },
      };
      requestPatch(payload as RegistryInput);
    }
  }

  return (
    <PlatformFormContext.Provider
      value={{
        mode,
        isLoading,
        isLoadingForm: createIsPending || patchIsPending,
        formTitle,
        saveButtonTitle,
        registry,
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
export { PlatformFormContext, PlatformFormProvider as default };
