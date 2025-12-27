using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class DeploymentRepository(IDbConnection db, Func<IDbTransaction> tx) : IDeploymentRepository
{

    public async Task<Deployment?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Deployments WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<DeploymentDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM Deployments WHERE name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Deployments (
                Id, Name, Description, PlatformId, Status, CreatedAt, CreatedByActorId, Spec
            ) VALUES (
                 @Id, @Name, @Description, PlatformId, @Status, @CreatedAt, @CreatedByActorId, @Spec
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id.Format(),
            Name = deployment.Name,
            Description = deployment.Description,
            PlatformId = deployment.PlatformId.Format(),
            CreatedByActorId = deployment.CreatedByActorId.Format(),
            CreatedAt = deployment.CreatedAt.ToString(),
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status),
            Spec = JsonSerializer.Serialize(deployment.Spec, DeploymentJsonContext.Default.DeploymentSpec)
        }, transaction: tx());
    }

    public async Task<IEnumerable<Deployment>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM Deployments";
        var result = await db.QueryAsync<DeploymentDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public Task<int> UpdateAsync(Deployment registry, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
