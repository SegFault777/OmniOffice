# New in 202647a

## OmniDoc
- **Tables**: Insert > Table 2x2/3x3/4x4. Tab / Shift+Tab move between cells (Tab in the last cell adds a row), Enter moves to the next cell, Insert > Table Edit adds or deletes rows and columns.
  Cells do not wrap: a row is one line. Tables are saved to OMD, DOCX, ODT and Markdown and imported from DOCX, ODT and DOC.
- **Pictures**: Insert > Picture (PNG, JPEG, GIF), saved in OMD and DOCX and read from DOCX.
- **Font formatting**: grow/shrink font, text colour (red, blue, green, automatic), strikethrough. Saved in OMD and DOCX.
- **Lists**: bullets and numbering (Ctrl+Shift+L for bullets), indent and outdent (Tab/Shift+Tab outside tables).
- **Spell check** (Review > Spelling, F7): red squiggles, right-click for suggestions and "Add to Dictionary". OmniDoc ships no dictionary. It reads the first word list it finds:
  `<config dir>/OmniOffice/dictionary.txt` (one word per line), `/usr/share/dict/words` and similar. Korean and other non-ASCII words are never flagged.
- **Find bar**: Shift+Enter replaces the current match and moves to the next.

## OmniShow
- **Pictures** (Insert > Picture), saved inside the .oms file. **Slide Number** toggle (Insert).
- **Shape Format** tab appears when a shape is selected: fill colours, bring forward/send backward, duplicate, align on the slide.
- **Presenter View** (Slide Show): current slide, next slide, notes and a timer.
- **Export PNG** (File): every slide as slide-NN.png at 1920x1080.

## OmniCel
- Ribbon: Home (undo, clipboard, bold, font colour, fill, alignment, number formats, AutoSum, fill down/right), Insert (column/line/pie charts), Data (sort), View.
- Number formats: General, 0.00, Percent, Currency, 1,000. Charts can be dragged to another cell; Delete removes the selected chart.
- Functions: SUMIF, COUNTIF, COUNTA, MEDIAN, AND, OR, NOT, CONCAT, LEN, UPPER, LOWER; $A$1 absolute references; Ctrl+D / Ctrl+R fill with reference adjustment; Alt+= AutoSum.
- Status bar shows Average, Count and Sum of the selected numbers. CSV files keep values and formulas only; use .omc for formats and charts.
