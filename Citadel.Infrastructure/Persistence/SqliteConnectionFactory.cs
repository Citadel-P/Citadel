using Microsoft.Data.Sqlite;

namespace Infrastructure.Persistence;

public interface IDbConnectionFactory
{
    SqliteConnection Create();
}

internal sealed class SqliteConnectionFactory : IDbConnectionFactory
{
    private readonly string _connectionString;

    public SqliteConnectionFactory(string connectionString)
    {
        _connectionString = connectionString;
    }

    public SqliteConnection Create()
    {
        var conn = new SqliteConnection(_connectionString);
        conn.Open();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = """
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA busy_timeout=3000;
        """;
        cmd.ExecuteNonQuery();

        return conn;
    }
}
