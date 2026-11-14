# UI design

Reference screenshots and research: Word (classic ribbon and 365 "Simplified" look), LibreOffice Writer
(Navigator, sidebar, several UI modes), OnlyOffice, WPS Office, FreeOffice. Press coverage in 2026 notes that
Word's ribbon suits intermediate users, WPS copies it most faithfully, and LibreOffice offers customisable layouts.

| Idea | Taken from | OmniDoc |
|---|---|---|
| Search box in the title bar | Word 365 | opens the command palette |
| Tabbed ribbon with groups, labelled shortcuts | Word, WPS | `File / Home / View`, generated from the command registry |
| Style gallery with live previews | Word | Heading buttons drawn in their own size/weight |
| Collapsible ribbon | Word (Ctrl+F1) | same |
| Navigation pane of headings | Word, LibreOffice Navigator | left pane, click to jump |
| Command palette | VS Code, Notion | Ctrl+Shift+P, fuzzy search |
| Find/replace bar, not a modal dialog | VS Code, Google Docs | bar under the ribbon |
| Focus mode, paper on neutral background | Word "Focus" | F11 |
| Status bar with word count and zoom | Word, LibreOffice | bottom bar |
| Dark theme | all modern suites | Ctrl+Shift+D |

Avoided: OnlyOffice's weak contrast and hidden style access, LibreOffice's dated icon density, cluttered default toolbars.

## 202646b additions
Right-click context menu (Word, LibreOffice), recent files in the File tab (Word backstage), ghost-text word completion
(LibreOffice Writer), find bar regex toggle (VS Code, LibreOffice), statistics dialog (LibreOffice Word Count).
OmniCel follows Excel/Hancom Cell (formula bar, name box, header-highlighted selection); OmniShow follows PowerPoint
(slide sorter, notes pane, F5 slideshow). Each app has its own accent colour: OmniDoc blue, OmniCel green, OmniShow orange.

## Icons (202646c)
Google Material Symbols Outlined (weight 400): one consistent family across Doc/Cel/Show, drawn as Skia paths so they scale with DPI and take the theme colour (accent when a toggle is on). Ribbon buttons follow Word: icon over label; shortcuts moved to menus/palette to reduce clutter.
