using Application.Features.Registries.Commands;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record CreateDeployment(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec) 
    : ICommand<Result<Deployment>>
{
    internal sealed class Validator : AbstractValidator<CreateRegistry>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }

    }
}

internal class CreateDeploymentHandler(IUnitOfWork unitOfWork) : ICommandHandler<CreateDeployment, Result<Deployment>>
{
    public ValueTask<Result<Deployment>> Handle(CreateDeployment command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
