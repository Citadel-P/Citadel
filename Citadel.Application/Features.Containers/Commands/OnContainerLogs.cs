using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Application.Features.Containers.Models;
using Application.Services.Abstractions;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record OnContainerLogs(ContainerLogRequest Input) : ICommand<Result>;

internal class OnContainerLogsHandler(IContainerHubDispatcher containerHub) : ICommandHandler<OnContainerLogs, Result>
{
    public async ValueTask<Result> Handle(OnContainerLogs command, CancellationToken cancellationToken)
    {
        await containerHub.SendContainerLogs(command.Input);
        return Result.Success();
    }
}