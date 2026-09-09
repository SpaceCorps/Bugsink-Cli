using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

/// <summary>
/// Settings for a command acting on one issue. The API resolves both forms itself, so the friendly
/// id printed in alerts and in the UI (e.g. MYPROJECT-7) works everywhere a UUID does.
/// </summary>
public class SingleIssueSettings : ApiSettings
{
    [CommandArgument(0, "<ISSUE>")]
    [Description("Issue UUID or friendly id, e.g. MYPROJECT-7")]
    public required string Issue { get; init; }

    public string Path(string action = "") => $"issues/{Uri.EscapeDataString(Issue)}/{action}";
}
