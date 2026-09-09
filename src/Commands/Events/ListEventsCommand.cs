using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Events;

public sealed class ListEventsCommand : AsyncCommand<ListEventsCommand.Settings>
{
    public sealed class Settings : PagedSettings
    {
        [CommandOption("--issue <ISSUE>")]
        [Description("Issue UUID or friendly id, e.g. MYPROJECT-7")]
        public required string Issue { get; init; }

        [CommandOption("--order <ORDER>")]
        [Description("asc or desc; defaults to desc (newest first)")]
        public string Order { get; init; } = "desc";
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var path = $"events/?issue={Uri.EscapeDataString(settings.Issue)}" +
                   $"&order={Uri.EscapeDataString(settings.Order)}";

        YamlOutput.Write(await client.GetPagedAsync(path, settings.EffectiveLimit));
        return 0;
    }
}
