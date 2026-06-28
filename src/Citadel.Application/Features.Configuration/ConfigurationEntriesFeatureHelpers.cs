using Application.Features.Configuration.Models;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Features.Configuration;

internal static class ConfigurationEntriesFeatureHelpers
{
    public static async Task<Result<ConfigurationEntriesResult>> GetResourceEntriesAsync(
        IUnitOfWork unitOfWork,
        ConfigurationScope scope,
        Guid resourceId,
        Func<Task<bool>> exists,
        CancellationToken cancellationToken)
    {
        if (!await exists())
            return Result.Failure<ConfigurationEntriesResult>(new NotFoundError($"{scope} with ID {resourceId} does not exist."));

        var entries = await unitOfWork.ConfigurationEntries.GetEntriesAsync(scope, resourceId, cancellationToken);
        var effective = await unitOfWork.ConfigurationEntries.GetEffectiveEntriesAsync(scope, resourceId, cancellationToken);
        return new ConfigurationEntriesResult([.. entries], [.. effective]);
    }

    public static async Task<Result<ConfigurationEntriesResult>> ReplaceEntriesAsync(
        IUnitOfWork unitOfWork,
        ConfigurationScope scope,
        Guid? resourceId,
        IReadOnlyList<ConfigurationEntryInput> inputs,
        CancellationToken cancellationToken)
    {
        var duplicate = inputs
            .GroupBy(x => x.Name, StringComparer.Ordinal)
            .FirstOrDefault(x => x.Count() > 1);

        if (duplicate is not null)
            return Result.Failure<ConfigurationEntriesResult>(new BadRequestError($"Configuration entry '{duplicate.Key}' is defined more than once."));

        var entries = inputs
            .Select(input => new ConfigurationEntry(
                Name: input.Name,
                Kind: input.Kind,
                Scope: scope,
                ResourceId: scope == ConfigurationScope.Global ? null : resourceId,
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
            return Result.Failure<ConfigurationEntriesResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.ConfigurationEntries.ReplaceEntriesAsync(scope, resourceId, entries, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var effective = scope == ConfigurationScope.Global
            ? entries
            : await unitOfWork.ConfigurationEntries.GetEffectiveEntriesAsync(scope, resourceId!.Value, cancellationToken);

        return new ConfigurationEntriesResult(entries, [.. effective]);
    }
}
