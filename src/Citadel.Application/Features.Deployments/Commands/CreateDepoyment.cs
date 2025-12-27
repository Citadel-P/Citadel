using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Deployments.Commands;

public sealed record CreateDeployment(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec) 
    : ICommand<Result<Deployment>>
{
    internal sealed class Validator : AbstractValidator<CreateDeployment>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);
        }
    }
}

internal class CreateDeploymentHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(CreateDeployment command, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var exist = await unitOfWork.Deployments.ExistsAsync(command.Name, cancellationToken);
        if (exist)
        {
            return Result.Failure<Deployment>(new ConflictError("Name already exists"));
        }

        var deployment = new Deployment(
            name : command.Name,
            description : command.Description,
            status : DeploymentStatus.Created,
            createdByActorId: user.GetActorId(),
            platformId : command.PlatformId,
            spec : command.Spec
            );

        var result = await unitOfWork.Deployments.AddAsync(deployment, cancellationToken);
        return deployment;
    }
}
