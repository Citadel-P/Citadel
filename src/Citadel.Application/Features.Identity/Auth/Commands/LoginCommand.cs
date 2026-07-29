using Application.Features.Identity.Auth.Models;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Auth.Commands;

public sealed record LoginCommand(string EmailOrName, string Password) : ICommand<Result<LoginResponse>>
{
    internal class Validator : AbstractValidator<LoginCommand>
    {
        public Validator()
        {
            RuleFor(x => x.EmailOrName).NotEmpty();
            RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);
        }
    }
}

internal sealed class LoginCommandHandler(
    IUnitOfWork unitOfWork,
    ICitadelPasswordHasher passwordHasher,
    ILocalAuthenticationCompletionService authenticationCompletion)
    : ICommandHandler<LoginCommand, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(LoginCommand query, CancellationToken cancellationToken)
    {
        var userAuthInfo = await unitOfWork.Users.GetUserAuthInfoByEmailOrNameAsync(query.EmailOrName, cancellationToken);
        if (userAuthInfo is null)
            return Result.Failure<LoginResponse>(new NotFoundError("Invalid credentials"));

        if (!passwordHasher.Verify(query.Password, userAuthInfo.Password ?? string.Empty))
            return Result.Failure<LoginResponse>(new BadRequestError("Invalid credentials"));

        return Result.Success(await authenticationCompletion.CompleteAsync(userAuthInfo, cancellationToken));
    }
}
