using System.Data;
using System.Data.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;

namespace Infrastructure.Persistence;

internal sealed class InstanceSetupStateRepository(
    IDbConnection db,
    Func<IDbTransaction> tx) : IInstanceSetupStateRepository
{
    private const string SelectColumns = """
        SELECT
            Id,
            InitializedAt,
            InitialAdministratorActorId,
            CreatedAt,
            UpdatedAt
        FROM InstanceSetupStates
        WHERE Id = @Id
        """;
    private const string SelectLocked = SelectColumns + " FOR UPDATE";

    public async Task<InstanceSetupState?> GetAsync(CancellationToken cancellationToken)
    {
        await using var command = CreateCommand(SelectColumns);
        AddParameter(command, "Id", InstanceSetupState.SingletonId);
        return await QuerySingleOrDefaultAsync(command, cancellationToken);
    }

    public async Task<InstanceSetupState?> GetLockedAsync(CancellationToken cancellationToken)
    {
        await using var command = CreateCommand(SelectLocked);
        AddParameter(command, "Id", InstanceSetupState.SingletonId);
        return await QuerySingleOrDefaultAsync(command, cancellationToken);
    }

    public async Task<int> UpdateAsync(
        InstanceSetupState state,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE InstanceSetupStates
            SET
                InitializedAt = @InitializedAt,
                InitialAdministratorActorId = @InitialAdministratorActorId,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
            """;

        await using var command = CreateCommand(sql);
        AddParameter(command, "Id", state.Id);
        AddParameter(command, "InitializedAt", state.InitializedAt?.UtcDateTime);
        AddParameter(
            command,
            "InitialAdministratorActorId",
            state.InitialAdministratorActorId);
        AddParameter(command, "UpdatedAt", state.UpdatedAt.UtcDateTime);
        return await command.ExecuteNonQueryAsync(cancellationToken);
    }

    private DbCommand CreateCommand(string commandText)
    {
        if (db is not DbConnection connection)
        {
            throw new InvalidOperationException(
                $"{nameof(InstanceSetupStateRepository)} requires a {nameof(DbConnection)}.");
        }

        if (tx() is not DbTransaction transaction)
        {
            throw new InvalidOperationException(
                $"{nameof(InstanceSetupStateRepository)} requires a {nameof(DbTransaction)}.");
        }

        var command = connection.CreateCommand();
        command.CommandText = commandText;
        command.Transaction = transaction;
        return command;
    }

    private static void AddParameter(
        DbCommand command,
        string name,
        object? value)
    {
        var parameter = command.CreateParameter();
        parameter.ParameterName = name;
        parameter.Value = value ?? DBNull.Value;
        command.Parameters.Add(parameter);
    }

    private static async Task<InstanceSetupState?> QuerySingleOrDefaultAsync(
        DbCommand command,
        CancellationToken cancellationToken)
    {
        await using var reader = await command.ExecuteReaderAsync(
            CommandBehavior.SingleRow,
            cancellationToken);
        if (!await reader.ReadAsync(cancellationToken))
        {
            return null;
        }

        return new InstanceSetupStateRow(
            reader.GetInt16(0),
            reader.IsDBNull(1) ? null : reader.GetDateTime(1),
            reader.IsDBNull(2) ? null : reader.GetGuid(2),
            reader.GetDateTime(3),
            reader.GetDateTime(4)).ToDomain();
    }
}

internal sealed record InstanceSetupStateRow(
    short Id,
    DateTime? InitializedAt,
    Guid? InitialAdministratorActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt)
{
    public InstanceSetupState ToDomain()
        => InstanceSetupState.FromPersistence(
            Id,
            InitializedAt is null
                ? null
                : new DateTimeOffset(InitializedAt.Value),
            InitialAdministratorActorId,
            new DateTimeOffset(CreatedAt),
            new DateTimeOffset(UpdatedAt));
}
