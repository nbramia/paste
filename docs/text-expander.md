# Text Expander Syntax Reference

Create snippets in the Snippets tab of the filmstrip. Each snippet has an **abbreviation** (trigger) and **content** (expansion template).

## Plain Text

The simplest snippet: type the abbreviation, get the content.

| Abbreviation | Content | Result |
|---|---|---|
| `;sig` | `Best regards,\nJohn Smith` | Best regards,<br>John Smith |
| `;email` | `john@example.com` | john@example.com |

## Date/Time Macros

Use `%` codes for dynamic date and time values.

| Macro | Output | Example |
|---|---|---|
| `%Y` | 4-digit year | 2026 |
| `%m` | Month (01-12) | 03 |
| `%d` | Day (01-31) | 22 |
| `%H` | Hour 24h (00-23) | 14 |
| `%M` | Minute (00-59) | 30 |
| `%S` | Second (00-59) | 45 |
| `%A` | Weekday name | Sunday |
| `%B` | Month name | March |
| `%p` | AM/PM | PM |
| `%%` | Literal `%` | % |

**Example:** `;today` → `Meeting notes for %Y-%m-%d` → `Meeting notes for 2026-03-22`

## Date Math

Use `%date(+/-Nunit)` for relative dates.

| Expression | Meaning |
|---|---|
| `%date(+5d)` | 5 days from now |
| `%date(-1w)` | 1 week ago |
| `%date(+3M)` | 3 months from now |
| `%date(+1y)` | 1 year from now |
| `%date(-2h)` | 2 hours ago |
| `%date(+30m)` | 30 minutes from now |

Units: `m` (minutes), `h` (hours), `d` (days), `w` (weeks), `M` (months), `y` (years).

Output format: `YYYY-MM-DD`.

**Example:** `;deadline` → `Due by %date(+14d)` → `Due by 2026-04-05`

## Clipboard

`%clipboard` inserts the current clipboard text content.

**Example:** `;quote` → `> %clipboard` → `> [whatever was copied]`

## Cursor Positioning

`%|` marks where the cursor should be placed after expansion.

**Example:** `;reply` → `Hi %|,\n\nThanks for your message.` — cursor lands after "Hi ".

## Fill-in Fields

Interactive fields that prompt for values before expanding.

| Syntax | Field Type |
|---|---|
| `%fill(name)` | Single-line text input |
| `%fill(name:default=value)` | Text with default value |
| `%fillarea(notes)` | Multi-line text area |
| `%fillpopup(tone:Professional:Casual:Formal)` | Dropdown selector |

**Example:** `;letter` →
```
Dear %fill(recipient),

%fillarea(body)

%fillpopup(closing:Best regards:Sincerely:Cheers),
%fill(name:default=John Smith)
```

A dialog appears to fill in the values before the text is inserted.

## Shell Commands

`%shell(command)` executes a shell command and uses its stdout as the expansion.

| Example | Output |
|---|---|
| `%shell(date +%s)` | Unix timestamp |
| `%shell(hostname)` | Machine hostname |
| `%shell(git branch --show-current)` | Current git branch |
| `%shell(curl -s wttr.in?format="%t")` | Current temperature |

Commands time out after 5 seconds. Errors produce `[shell error: ...]` text.

## Nested Snippets

`%snippet(abbreviation)` expands another snippet inline.

**Example:** If `;phone` expands to `555-1234`, then:

`;contact` → `John Smith\nPhone: %snippet(;phone)` → `John Smith\nPhone: 555-1234`

- Max nesting depth: 10
- Circular references are detected and produce an error
- The evaluation context (clipboard, date, etc.) is shared across the chain

## Trigger Modes

Configure in Settings or `config.toml`:

- **Word boundary** (default): Expansion triggers when you type the abbreviation followed by a space, tab, or punctuation. Safest — no accidental triggers.
- **Immediate**: Expansion triggers as soon as the abbreviation is fully typed. Faster but more prone to false positives with short abbreviations.

## Importing from espanso

Click "Import espanso" in the Snippets tab to import snippets from `~/.config/espanso/match/*.yml`. Variable mapping:

| espanso | Paste |
|---|---|
| `{{clipboard}}` | `%clipboard` |
| `{{date}}` | `%Y-%m-%d` |
| `{{time}}` | `%H:%M:%S` |
| `{{newline}}` | `\n` |

Duplicate abbreviations are automatically skipped during import.

## JSON Export/Import

Export all snippets and groups to a JSON file for backup or transfer:

- **Export**: Saves all snippets organized by group, including abbreviations, content, types, and descriptions
- **Import**: Reads a previously exported JSON file. Duplicate abbreviations are skipped. Groups are created or matched by name.

When importing, snippets containing `%shell(...)` macros are flagged with a warning since they execute arbitrary commands. Review imported script snippets before using them.

## Tips for Abbreviations

- **Use a prefix**: Starting all abbreviations with a symbol (`;`, `//`, `,,`) avoids accidental triggers. For example: `;sig`, `;date`, `;addr`.
- **Keep them memorable**: `;sig` for signature, `;em` for email, `;ph` for phone.
- **Avoid common words**: Don't use abbreviations that could appear in normal typing.
- **Use word boundary mode**: The default trigger mode prevents false positives by requiring a space or punctuation after the abbreviation.
