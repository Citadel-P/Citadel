using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services.Identity;

internal interface IAdministratorGuard
{
    Task<Result> EnsureAdministratorRemainsAsync(CancellationToken cancellationToken);
}

internal sealed class AdministratorGuard(IUnitOfWork unitOfWork) : IAdministratorGuard
{
    public async Task<Result> EnsureAdministratorRemainsAsync(CancellationToken cancellationToken)
    {
        if (await unitOfWork.Users.HasEnabledAdministratorAsync(cancellationToken))
            return Result.Success();

        await unitOfWork.RollbackAsync();
        return Result.Failure(new ConflictError("Citadel must have at least one enabled administrator."));
    }
}
