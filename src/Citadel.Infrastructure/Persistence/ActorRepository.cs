using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class ActorRepository(IDbConnection db, Func<IDbTransaction> tx) : IActorRepository
{
    public Task<int> AddAsync(Actor actor, CancellationToken cancellationToken)
    {
        const string sql = "INSERT INTO Actors (Id, Type, IsEnabled) VALUES (@Id, @Type, @IsEnabled)";
        return db.ExecuteAsync(sql, new
        {
            Id = actor.Id,
            Type = EnumFormatter<ActorType>.GetValue(actor.Type),
            actor.IsEnabled,
            cancellationToken
        }, transaction: tx());
    }

    public async Task<Actor?> GetById(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                Actors.Id,
                Actors.Type,
                COALESCE(Users.Name, Teams.Name, CASE WHEN Actors.Type = 'System' THEN 'System' END) AS Name,
                Actors.IsEnabled AS IsEnabled
            FROM Actors
            LEFT JOIN Users ON Users.ActorId = Actors.Id
            LEFT JOIN Teams ON Teams.ActorId = Actors.Id
            WHERE Actors.Id = @Id
            LIMIT 1
            """;
        var result = await db.QuerySingleOrDefaultAsync<ActorDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<int> UpdateAsync(Actor actor, CancellationToken cancellationToken)
    {
        const string sql = "UPDATE Actors SET IsEnabled = @IsEnabled WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = actor.Id, IsEnabled = actor.IsEnabled, cancellationToken }, transaction: tx());
    }
}
