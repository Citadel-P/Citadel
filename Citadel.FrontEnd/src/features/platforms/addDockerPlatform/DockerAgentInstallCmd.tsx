import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { CodeBlock } from 'react-code-block';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { Clipboard, CheckCheck } from 'lucide-react';

const DockerAgentInstallCmd = () => {
  const [copiedWinCmd, copyWinCmdToClipboard] = useCopyToClipboard(5000);
  const [copiedLinuxCmd, copyLinuxCmdToClipboard] = useCopyToClipboard(5000);

  const agentVersion = '1.0.0';

  const commands = [
    {
      name: 'linuxCommand',
      isCopied: copiedLinuxCmd,
      copyHandler: copyLinuxCmdToClipboard,
      code: `
docker run -d
    -p 8001:8001 
    --name Citadel_agent 
    --restart=always
    -v /var/run/docker.sock:/var/run/docker.sock
    -v /var/lib/docker/volumes:/var/lib/docker/volumes
    Citadel/agent:${agentVersion}
            `,
    },
    {
      name: 'windowsCommand',
      isCopied: copiedWinCmd,
      copyHandler: copyWinCmdToClipboard,
      code: `
docker run -d 
    -p 8001:8001 
    --name Citadel_agent
    --restart=always
    -v C:\\ProgramData\\docker\\volumes:C:\\ProgramData\\docker\\volumes 
    -v .\\pipe\\docker_engine:\\.\\pipe\\docker_engine
    Citadel/agent:${agentVersion}
`,
    },
  ];
  return (
    <Tabs defaultValue="linuxCommand">
      <TabsList className="w-full justify-start bg-muted/20 rounded-sm">
        <TabsTrigger value="linuxCommand">Linux and Windows WSL</TabsTrigger>
        <TabsTrigger value="windowsCommand">Windows WCS</TabsTrigger>
      </TabsList>
      {commands.map((cmd, index) => (
        <TabsContent key={index} value={cmd.name}>
          <CodeBlock code={cmd.code} language="javascript">
            <div className="relative">
              <CodeBlock.Code className="bg-card-foreground dark:bg-card !p-6 rounded-sm shadow-sm">
                <div className="table-row">
                  <CodeBlock.LineNumber className="table-cell pr-4 text-xs text-gray-500 text-right select-none" />
                  <CodeBlock.LineContent className="table-cell">
                    <CodeBlock.Token />
                  </CodeBlock.LineContent>
                </div>
              </CodeBlock.Code>
              <button
                className="bg-background rounded-md px-1.5 py-1.5 absolute top-2 right-2 text-sm font-semibold"
                onClick={() => cmd.copyHandler(cmd.code)}>
                {cmd.isCopied ? <CheckCheck className="w-4 h-4 text-green-500" /> : <Clipboard className="w-4 h-4" />}
              </button>
            </div>
          </CodeBlock>
        </TabsContent>
      ))}
    </Tabs>
  );
};

export default DockerAgentInstallCmd;
