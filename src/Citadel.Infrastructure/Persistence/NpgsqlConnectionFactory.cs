using Npgsql;
using System.Data.Common;

namespace Infrastructure.Persistence;

public interface IDbConnectionFactory
{
    DbConnection Create();
}

internal sealed class NpgsqlConnectionFactory : IDbConnectionFactory
{
    private readonly NpgsqlDataSource _dataSource;

    public NpgsqlConnectionFactory(NpgsqlDataSource dataSource)
    {
        _dataSource = dataSource;
    }

    public DbConnection Create() => _dataSource.CreateConnection();
}