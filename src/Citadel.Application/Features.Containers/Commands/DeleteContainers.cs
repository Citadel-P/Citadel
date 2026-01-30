using Application.Services;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record DeleteContainers(string[] ContainerIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerIds).NotEmpty().WithMessage("At least one container ID must be provided.");
            RuleForEach(s => s.ContainerIds).ValidContainerId();
        }
    }
}

internal sealed class DeleteContainersHandler(IContainerProcessingService containerService): ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken ct)
    {
        return await containerService.DeleteContainers(request, ct);
    }

}