using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Infrastructure.TypeHandlers;
using Microsoft.AspNetCore.Http.HttpResults;
using System;
using System.Collections.Generic;
using System.Data;
using System.Text;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal class DeploymentRepository(IDbConnection db, Func<IDbTransaction> tx) : IDeploymentRepository
{
    public Task<int> AddAsync(Deployment deployment, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO Deployments (
                Id, Name, Description, CreatedAt, CreatedBy, Status
            ) VALUES (
                 @Id, @Name, @Description, @CreatedAt, @UpdatedAt, @CreatedBy, @Status
            )
        """;
        return db.ExecuteAsync(sql, new
        {
            Id = deployment.Id.Format(),
            Name = deployment.Name,
            Description = deployment.Description,
            CreatedBy = deployment.CreatedBy.Format(),
            CreatedAt = deployment.CreatedAt.ToString(),
            Status = EnumFormatter<DeploymentStatus>.GetValue(deployment.Status)
        }, transaction: tx());
    }
}
