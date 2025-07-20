using System.Data;
using Dapper;

namespace Infrastructure.TypeHandlers;

public sealed class GuidStringHandler : SqlMapper.TypeHandler<Guid>
{
    public override Guid Parse(object value)
    {
        // Avoid unnecessary boxing or .ToString() allocations
        return value is string str && Guid.TryParse(str, out var guid)
            ? guid
            : throw new FormatException("Invalid GUID format in SQLite text column.");
    }

    public override void SetValue(IDbDataParameter parameter, Guid value)
    {
        // Write as text (SQLite-safe) without ToString() if possible
        Span<char> buffer = stackalloc char[36];
        if (value.TryFormat(buffer, out int charsWritten))
        {
            parameter.Value = buffer[..charsWritten].ToString();
        }
        else
        {
            parameter.Value = value.ToString();
        }
    }
}


