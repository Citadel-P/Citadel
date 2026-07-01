using Application.Features.Configuration.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Features.Configuration;

internal static class ResourceBindingsFeatureHelpers
{
    public static async Task<Result<ResourceBindingsResult>> GetResourceEntriesAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid resourceId,
        Func<Task<bool>> exists,
        CancellationToken cancellationToken)
    {
        if (!await exists())
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"{scope} with ID {resourceId} does not exist."));

        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(scope, resourceId, cancellationToken);
        var effective = await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId, cancellationToken);
        return new ResourceBindingsResult([.. entries], [.. effective]);
    }

    public static async Task<Result<ResourceBindingsResult>> ReplaceEntriesAsync(
        IUnitOfWork unitOfWork,
        ResourceBindingScope scope,
        Guid? resourceId,
        IReadOnlyList<ResourceBindingInput> inputs,
        CancellationToken cancellationToken)
    {
        var duplicate = inputs
            .GroupBy(x => x.Name, StringComparer.Ordinal)
            .FirstOrDefault(x => x.Count() > 1);

        if (duplicate is not null)
            return Result.Failure<ResourceBindingsResult>(new BadRequestError($"Resource binding '{duplicate.Key}' is defined more than once."));

        var entries = inputs
            .Select(input => new ResourceBinding(
                Name: input.Name,
                Kind: input.Kind,
                Scope: scope,
                ResourceId: scope == ResourceBindingScope.Global ? null : resourceId,
                Value: input.Value,
                SecretId: input.SecretId,
                SecretDeliveryMode: input.SecretDeliveryMode,
                TargetPath: input.TargetPath))
            .ToArray();

        try
        {
            foreach (var entry in entries)
                entry.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<ResourceBindingsResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.ResourceBindings.ReplaceEntriesAsync(scope, resourceId, entries, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var effective = scope == ResourceBindingScope.Global
            ? entries
            : await unitOfWork.ResourceBindings.GetEffectiveEntriesAsync(scope, resourceId!.Value, cancellationToken);

        return new ResourceBindingsResult(entries, [.. effective]);
    }
}
