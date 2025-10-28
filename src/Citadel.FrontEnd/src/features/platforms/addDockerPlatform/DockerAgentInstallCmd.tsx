import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import CodeHighlight from '@/components/custom/code-highlight';

const DockerAgentInstallCmd = () => {
  const agentVersion = '1.0.0';

  const commands = [
    {
      name: 'linuxCommand',
      code: `docker run -d
    -p 9000:9000 
    --name Citadel_agent 
    --restart=always
    -v /var/run/docker.sock:/var/run/docker.sock
    -v /var/lib/docker/volumes:/var/lib/docker/volumes
    Citadel/agent:${agentVersion}`,
    },
    {
      name: 'windowsCommand',
      code: `docker run -d 
    -p 9000:9000 
    --name Citadel_agent
    --restart=always
    -v C:\\ProgramData\\docker\\volumes:C:\\ProgramData\\docker\\volumes 
    -v .\\pipe\\docker_engine:\\.\\pipe\\docker_engine
    Citadel/agent:${agentVersion}`,
    },
  ];

  return (
    <Tabs defaultValue="linuxCommand">
      <TabsList className="w-full">
        <TabsTrigger value="linuxCommand">Linux and Windows WSL</TabsTrigger>
        <TabsTrigger value="windowsCommand">Windows WCS</TabsTrigger>
      </TabsList>
      {commands.map((cmd, index) => (
        <TabsContent key={index} value={cmd.name}>
          <CodeHighlight
            code={cmd.code}
            language="tsx"
            className="bg-card-foreground dark:bg-card p-6! rounded-sm shadow-xs relative"
            lineContentClassName="text-md"
            showCopyButton
          />
        </TabsContent>
      ))}
    </Tabs>
  );
};

export default DockerAgentInstallCmd;
