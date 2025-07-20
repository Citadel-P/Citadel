using System.Text;
using Dapper;

namespace Infrastructure.TypeHandlers;

/// <summary>
/// SQLite does not support binding a collection directly via IN @Param. 
/// Unlike PostgreSQL (which supports array expansion) or SQL Server (which supports Table-Valued Parameters), 
/// SQLite expects a fixed list of scalar values in the IN clause.
/// https://github.com/DapperLib/DapperAOT/issues/160
/// </summary>
public static class SqliteInClauseBuilder
{
    public static (string SqlClause, DynamicParameters Parameters) BuildInClauseForGuids(string paramPrefix, IEnumerable<Guid> guids)
    {
        var parameters = new DynamicParameters();
        var sb = new StringBuilder();
        int index = 0;

        foreach (var guid in guids)
        {
            if (index > 0)
                sb.Append(", ");

            var paramName = $"{paramPrefix}{index}";
            sb.Append('@').Append(paramName);
            parameters.Add(paramName, guid.Format());

            index++;
        }

        return (sb.ToString(), parameters);
    }

    public static (string clause, DynamicParameters parameters) BuildInClauseForStrings(string baseName, IEnumerable<string> values)
    {
        var parameters = new DynamicParameters();
        var clauseBuilder = new StringBuilder();
        int i = 0;

        foreach (var value in values)
        {
            if (string.IsNullOrEmpty(value))
                continue;

            if (i > 0) clauseBuilder.Append(", ");
            string paramName = $"{baseName}{i}";
            clauseBuilder.Append('@').Append(paramName);
            parameters.Add(paramName, value);
            i++;
        }

        // If no valid values, produce safe false clause (e.g., IN (NULL))
        if (i == 0)
        {
            return ("NULL", new DynamicParameters()); // Empty clause => safe no-match
        }

        return (clauseBuilder.ToString(), parameters);
    }
}
