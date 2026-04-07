using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, ResourceAction.Create)]
public sealed record CreateUser(string Name, string Email, string Password) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : AbstractValidator<CreateUser>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Email).NotEmpty().EmailAddress();
            RuleFor(x => x.Password).NotNull().MinimumLength(8).MaximumLength(128);
        }
    }
}

internal sealed class CreateUserHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(CreateUser command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
            ?? throw new ArgumentNullException("ActorId claim is missing");

        var conflicts = await unitOfWork.Users.GetConflictsAsync(command.Name, command.Email, null, cancellationToken);
        if (conflicts.NameExists)
            return Result.Failure<UserDetails>(new ConflictError("Name already exists"));

        if (conflicts.EmailExists)
            return Result.Failure<UserDetails>(new ConflictError("Email already exists"));

        var userActor = Actor.Create(ActorType.User, new ActorMetadata(command.Name));
        var user = new User(command.Name, command.Email, command.Password, userActor.Id, actorId);

        await unitOfWork.Actors.AddAsync(userActor, cancellationToken);
        await unitOfWork.Users.AddAsync(user, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new UserDetails(user.Id, user.Name, user.Email, user.ActorId, userActor.IsEnabled, user.CreatedAt, user.CreatedByActorId);
    }
}
