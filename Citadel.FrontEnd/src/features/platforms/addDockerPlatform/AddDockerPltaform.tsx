import DockerIcon from '@/assets/docker.min.svg';
import DockerAgentInstallCmd from './DockerAgentInstallCmd';
import ConnectAgentForm from './ConnectAgentForm';
import { useHideBreadcrumb } from '@/layout/breadcrumb/useHideBreadcrumb';

const AddDockerPlatform = () => {
  useHideBreadcrumb();
  return (
    <div className="mx-auto px-4 py-3 xl:w-2/3 sm:px-6">
      <div className="mb-3 flex items-baseline gap-1">
        <div className="inline-flex h-8 w-8 bg-primary/10 text-primary shrink-0 items-center justify-center rounded-lg ">
          <span className="h-4 w-4 ">
            <DockerIcon />
          </span>
          <span className="sr-only">New Platform</span>
        </div>
        <div className="text-md font-bold text-foreground">New Platform</div>
        <div className="text-xs text-muted-foreground">(Docker Standalone)</div>
      </div>

      <div className=" rounded-lg border-border bg-background p-4 relative">
        <ol className="relative border-s border-border ml-1">
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              1
            </span>
            <h2 className="text-sm mb-2 font-semibold text-foreground">
              Execute the below command on your Docker Standalone environment
            </h2>
            <DockerAgentInstallCmd />
          </li>
          <li className="mb-10 ms-6">
            <span className="absolute flex items-center justify-center w-5 h-5 bg-primary/80 rounded-full -start-2.5 ring-4 ring-background text-xs text-primary-foreground">
              2
            </span>
            <h2 className="text-sm font-semibold text-foreground">Connect</h2>
            <ConnectAgentForm />
          </li>
        </ol>
      </div>
    </div>
  );
};

export default AddDockerPlatform;
