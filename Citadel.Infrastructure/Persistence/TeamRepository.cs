using System.Data;
using Domain.Contracts.Interfaces;

namespace Infrastructure.Persistence;

internal class TeamRepository(IDbConnection db, IDbTransaction tx) : ITeamRepository { }
