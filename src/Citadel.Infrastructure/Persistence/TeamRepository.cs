using System.Data;
using Domain.Contracts.Interfaces;
using Microsoft.Data.Sqlite;

namespace Infrastructure.Persistence;

internal class TeamRepository(IDbConnection db, Func<IDbTransaction> tx) : ITeamRepository { }
