using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Teams;

public sealed class ListTeamsCommand : AsyncCommand<PagedSettings>
{
    public override async Task<int> ExecuteAsync(CommandContext context, PagedSettings settings)
    {
        var client = settings.CreateClient();
        YamlOutput.Write(await client.GetPagedAsync("teams/", settings.EffectiveLimit));
        return 0;
    }
}
