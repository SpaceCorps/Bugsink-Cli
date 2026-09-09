using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Teams;

public sealed class CreateTeamCommand : AsyncCommand<CreateTeamCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandOption("--name <NAME>")]
        [Description("Team name")]
        public required string Name { get; init; }

        [CommandOption("--visibility <VISIBILITY>")]
        [Description("joinable, discoverable or hidden; defaults to discoverable")]
        public string? Visibility { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        var body = new Dictionary<string, object?> { ["name"] = settings.Name };
        if (settings.Visibility is not null) body["visibility"] = settings.Visibility;

        YamlOutput.Write(await client.PostAsync("teams/", body));
        Status.Ok($"Team {settings.Name} created.");
        return 0;
    }
}
