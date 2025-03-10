import { CreateRegistryInput, PatchRegistryInput, RegistryDiscriminator, RegistryView } from '@/api/_generated';
import { createContext } from 'use-context-selector';
import { JSX, useEffect, useState } from 'react';
import { toast } from 'sonner';
import DockerHubConfiguration from './DockerHubConfiguration';
import GhcrConfiguration from './GhcrConfiguration';
import { useNavigate, useParams } from 'react-router';
import { useGetRegistry } from './hooks/useGetRegistry';
import { usePOSTRegistry } from './hooks/usePOSTRegistry';
import { usePATCHRegistry } from './hooks/usePATCHRegistry';

interface IRegistryProvider {
  id: string;
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
  registry: RegistryView | undefined;
  providers: IRegistryProvider[];
  currentProvider: string;
  setCurrentProvider: (value: string) => void;
  onPostForm: (values: CreateRegistryInput | Partial<CreateRegistryInput>) => void;
}
interface IProps {
  children?: React.ReactNode;
}

const RegistryFormContext = createContext<IContext | undefined>(undefined);

const RegistryFormProvider: React.FC<IProps> = ({ children }) => {
  const defaultProviders: IRegistryProvider[] = [
    {
      id: RegistryDiscriminator.DockerHub,
      name: 'DockerHub',
      description: 'Docker hub authenticated account',
      configuration: <DockerHubConfiguration />,
      disabled: false,
    },
    {
      id: RegistryDiscriminator.GitHub,
      name: 'GitHub',
      description: 'GitHub container registry Ghcr',
      configuration: <GhcrConfiguration />,
      disabled: false,
    },
    {
      id: RegistryDiscriminator.AWS,
      name: 'AWS ECR',
      description: 'Amazon elastic container registry',
      configuration: <>Amazon Cfg</>,
      disabled: true,
    },
    {
      id: RegistryDiscriminator.Gitlab,
      name: 'Gitlab',
      description: 'GitLab container registry',
      configuration: <>Gitlab Cfg</>,
      disabled: true,
    },
  ] as const;

  const navigate = useNavigate();
  const { registryId } = useParams();
  const mode: FormMode = registryId ? 'edit' : 'add';
  const { data, isLoading } = useGetRegistry(registryId);
  const {
    mutate: requestCreate,
    validationErrors: createErrors,
    isSuccess: createIsSuccess,
    isPending: createIsPending,
    data: createData,
  } = usePOSTRegistry();
  const {
    mutate: requestPatch,
    validationErrors: patchErrors,
    isSuccess: patchIsSuccess,
    isPending: patchIsPending,
    data: patchData,
  } = usePATCHRegistry();
  const [currentProvider, setCurrentProvider] = useState<string>(RegistryDiscriminator.DockerHub);
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
      setCurrentProvider(data?.data.discriminator);
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

  function onPostForm(data: CreateRegistryInput | Partial<PatchRegistryInput>) {
    if (mode === 'add') {
      requestCreate(data as CreateRegistryInput);
    } else {
      // for serialization
      const payload = {
        ...data,
        id: registry?.id,
        discriminator: registry?.discriminator,
        configuration: { $type: registry?.configuration.$type, ...data.configuration },
      };
      requestPatch(payload as PatchRegistryInput);
    }
  }

  return (
    <RegistryFormContext.Provider
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
        onPostForm,
      }}>
      {children}
    </RegistryFormContext.Provider>
  );
};

type FormMode = 'edit' | 'add';
export { RegistryFormContext, RegistryFormProvider as default };
