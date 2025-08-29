import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { Clipboard, CheckCheck } from 'lucide-react';
import { Highlight, themes } from 'prism-react-renderer';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

const DockerAgentInstallCmd = () => {
  const [copiedWinCmd, copyWinCmdToClipboard] = useCopyToClipboard(5000);
  const [copiedLinuxCmd, copyLinuxCmdToClipboard] = useCopyToClipboard(5000);

  const agentVersion = '1.0.0';

  const commands = [
    {
      name: 'linuxCommand',
      isCopied: copiedLinuxCmd,
      copyHandler: copyLinuxCmdToClipboard,
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
      isCopied: copiedWinCmd,
      copyHandler: copyWinCmdToClipboard,
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
          <Highlight theme={themes.nightOwl} code={cmd.code} language="tsx">
            {({ style, tokens, getLineProps, getTokenProps }) => (
              <pre style={style} className="bg-card-foreground dark:bg-card p-6! rounded-sm shadow-xs relative">
                {tokens.map((line, i) => (
                  <div key={i} {...getLineProps({ line })} className="table-row">
                    <span className="table-cell pr-4 text-xs text-gray-500 text-right select-none">{i + 1}</span>
                    {line.map((token, key) => (
                      <span key={key} {...getTokenProps({ token })} className="text-md" />
                    ))}
                  </div>
                ))}
                <TooltipProvider delayDuration={200}>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <button
                        className="rounded-md px-1.5 py-1.5 absolute top-2 right-2 text-sm font-semibold"
                        onClick={() => cmd.copyHandler(cmd.code)}>
                        {cmd.isCopied ? (
                          <CheckCheck className="w-4 h-4 text-green-500" />
                        ) : (
                          <Clipboard className="w-4 h-4 " />
                        )}
                      </button>
                    </TooltipTrigger>
                    <TooltipContent>Copy</TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </pre>
            )}
          </Highlight>
        </TabsContent>
      ))}
    </Tabs>
  );
};

export default DockerAgentInstallCmd;
