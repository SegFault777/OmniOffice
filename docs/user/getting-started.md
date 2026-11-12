# Getting started

OmniDoc is a terminal application (a UTF-8 capable terminal is required; Windows Terminal or
a recent conhost on Windows).

1. Run `omnidoc notes.md` (the file is created on first save if it does not exist).
2. Type. Lines are soft-wrapped; CJK characters are treated as double width.
3. `^S` saves in the format given by the file extension. Read-only formats are saved as `.omd`.
4. `^F` searches, `^Z` undoes, `^Q` quits (press twice with unsaved changes).

The top bar shows the Word-style tabs (File, Home, Insert, View) as placeholders for the upcoming
ribbon; the bottom bar shows line, column and word count.

Limits: documents up to 4 MiB; no formatting is preserved when converting between formats.
Headless conversion: `omnidoc --convert in.docx out.md`.
