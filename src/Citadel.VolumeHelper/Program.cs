using Hosting.DockerClient.VolumeHelper;

var commandArgs = args.Length > 0 && args[0] == "volume-helper"
    ? args[1..]
    : args;

return await VolumeHelperCommand.RunAsync(
    commandArgs,
    Console.OpenStandardOutput(),
    Console.Error,
    CancellationToken.None);
