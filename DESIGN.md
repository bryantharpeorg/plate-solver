---
name: Finder
description: The finder console, a field atlas of your own frame, in a day desk edition and a red-on-black night edition.
colors:
  paper: "oklch(98.4% 0.003 250)"
  paper-2: "oklch(95.8% 0.005 250)"
  ink: "oklch(19% 0.02 262)"
  ink-2: "oklch(41% 0.02 262)"
  ink-3: "oklch(60% 0.015 262)"
  rule: "oklch(86.5% 0.008 262)"
  nebula: "oklch(49% 0.12 158)"
  cluster: "oklch(57% 0.13 76)"
  galaxy: "oklch(53% 0.19 27)"
  alert: "oklch(53% 0.19 27)"
  photo: "oklch(13% 0.01 262)"
  on-photo: "oklch(95% 0.012 95)"
  on-photo-2: "oklch(80% 0.015 95)"
  grid: "oklch(84% 0.03 250 / 0.42)"
  nebula-photo: "oklch(81% 0.14 158)"
  cluster-photo: "oklch(87% 0.14 88)"
  galaxy-photo: "oklch(74% 0.16 27)"
  night-ground: "#000000"
  night-ground-2: "#140000"
  night-ink: "#e00000"
  night-ink-2: "#a80000"
  night-ink-3: "#760000"
  night-rule: "#3c0000"
  night-grid: "rgb(168 0 0 / 0.6)"
typography:
  display:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "2.75rem"
    fontWeight: 680
    lineHeight: 1
    letterSpacing: "-0.02em"
    fontFeature: "\"tnum\" 1"
  chart-title:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.2rem"
    fontWeight: 640
    letterSpacing: "0.3em"
    fontVariation: "\"wdth\" 118"
  headline:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 650
    letterSpacing: "-0.01em"
  title:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 600
  body:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "\"tnum\" 1"
  body-quiet:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 600
    letterSpacing: "0.12em"
  object-label:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 620
  edge-label:
    fontFamily: "Archivo Variable, Archivo, ui-sans-serif, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 520
    fontVariation: "\"wdth\" 82"
rounded:
  none: "0px"
  focus: "3px"
  skeleton: "4px"
  sheet: "12px"
spacing:
  double-rule: "3px"
  chip-x: "10px"
  button-x: "16px"
  frame-x: "14px"
  frame-x-wide: "20px"
  row-y: "10px"
  chart-margin: "30px"
  column-gap: "40px"
components:
  button:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0 16px"
    height: "44px"
  button-hover:
    backgroundColor: "{colors.paper-2}"
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.none}"
    padding: "0 16px"
    height: "44px"
  button-primary-hover:
    backgroundColor: "{colors.ink-2}"
  button-primary-night:
    backgroundColor: "{colors.night-rule}"
    textColor: "{colors.night-ink}"
  chip:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0 10px"
    height: "38px"
  chip-off:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
  switch:
    backgroundColor: "transparent"
    rounded: "{rounded.none}"
    width: "44px"
    height: "24px"
  switch-on:
    backgroundColor: "{colors.rule}"
  input:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.none}"
    padding: "0 12px"
    height: "44px"
  list-row:
    textColor: "{colors.ink}"
    padding: "10px 0"
  list-row-selected:
    backgroundColor: "{colors.paper-2}"
  sheet:
    backgroundColor: "{colors.paper}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sheet}"
    width: "640px"
---

# Design System: Finder

## Overview

**Creative North Star: "The Field Atlas"**

The console is an atlas chart of the observer's own frame. The solve proves itself by drawing the sky's RA/Dec graticule and the atlas's deep-sky symbols over the real photograph, labelled at the edges in a paper margin the way a printed star atlas labels a chart. Everything around the chart is set like the atlas's own furniture: a double-ruled title block, hairline rules between entries, square ruled boxes for controls, black ink on cool paper.

The system ships in two editions of the same chart. The day edition is the desk edition: cool paper, near-black ink in three strengths, hairline rules, and the atlas's object colours (green nebulae, gold clusters, red galaxies) kept to object symbols and labels. The night edition is the field edition read under a red torch: black ground and red ink in three brightnesses, every token on the red channel alone, and photographs passed through a luminance-to-red filter so not one white pixel reaches the screen. Density is that of a reference chart: dense facts, tabular figures, few surfaces, no decoration. The confirmed rejection is the planetarium-app default of navy glass cards, cyan glow and starfield wallpaper.

**Key Characteristics:**
- An RA/Dec graticule and atlas symbols drawn over the real frame are the signature.
- Square ruled boxes in 1px ink; the only rounded thing is the dialog sheet.
- Flat: no shadows anywhere; depth is rule weight and tone.
- One family, Archivo Variable, worked through its width axis and italic.
- Two editions from one token set: day paper and ink, night red on black.

## Colors

A paper-and-ink palette with three atlas object colours, remapped wholesale to red on black at night.

### Primary
- **Chart Ink** (`ink`): text, every 1px ruled box, the chart's neatline and margin ticks, the primary button fill, focus outlines and text selection.

### Secondary
- **Nebula Green** (`nebula`, `nebula-photo` over the frame): diffuse, planetary and supernova-remnant nebula symbols and labels.
- **Cluster Gold** (`cluster`, `cluster-photo`): open and globular cluster and star-group symbols.
- **Galaxy Red** (`galaxy`, `galaxy-photo`): galaxy ellipses and labels.

The paper variants sit on paper in lists; the `-photo` variants are lightened to read over the dark frame.

### Tertiary
- **Alert Red** (`alert`): the error rule and its heading, the no-match warning icon. It shares Galaxy Red's value but is its own role.

### Neutral
- **Cool Paper** (`paper`): page ground, chips, the top bar, sheets.
- **Shaded Paper** (`paper-2`): hover fills, the selected list row, thumbnail placeholders.
- **Second Ink** (`ink-2`): secondary text, units, group titles, quiet notes, edge labels, an off chip.
- **Third Ink** (`ink-3`): dimmed symbols for targets below the horizon, scrollbar thumb.
- **Hairline** (`rule`): dividers between list rows, the top bar's bottom rule, the pressed fill, the switch's on track.
- **Frame Black** (`photo`): the ground under and around the photograph, label casings.
- **Frame Ink** (`on-photo`, `on-photo-2`): matched-star rings and names, and the compass and scale furniture (full); expected stars and detected centroids (second).
- **Graticule** (`grid`): RA/Dec lines over the frame, translucent.

### Night edition
Under `[data-theme="night"]` every token above is reassigned. `paper` becomes Night Ground (`night-ground`), `paper-2` becomes `night-ground-2`, the inks become Night Ink, Second and Third (`night-ink`, `night-ink-2`, `night-ink-3`), and `rule` becomes `night-rule`. All three object colours, `alert`, `on-photo` and the object `-photo` colours collapse to Night Ink, and `grid` becomes `night-grid`. Object families are then told apart by symbol shape alone. Photographs, thumbnails and the survey view take the `#red-only` SVG filter (luminance into the red channel, scaled so white lands just under Night Ink), and the pre-paint script and inline style set a black ground before any stylesheet loads.

### Named Rules
**The Objects-Only Colour Rule.** Green, gold and red belong to deep-sky objects. Chrome, controls and text stay in ink.

**The Red Torch Rule.** At night every pixel is red or black: no white, no other hue, and no token brighter than Night Ink. Anything new, images and embedded views included, must take a night token or the red-only filter before it ships.

**The Dim Primary Rule.** At night the primary button is a Night Rule block with Night Ink text, never a solid Night Ink fill. A full-ink block would be the brightest thing on screen.

## Typography

**Display Font:** Archivo Variable (with Archivo, ui-sans-serif, system-ui, sans-serif)
**Body Font:** Archivo Variable
**Label/Mono Font:** Archivo Variable, with the `wdth` axis condensed for chart edge labels and tabular figures on everywhere

**Character:** A single grotesque used the way an atlas uses one face: expanded and letterspaced for the chart title, heavy and tight for coordinates, italic for object names, condensed for the margin scale. Self-hosted, so it renders offline.

### Hierarchy
- **Display** (680, 2.75rem on wide screens and 2.35rem on phones, line-height 1, -0.02em): RA and Dec. Unit marks (h m s, ° ′ ″) are set at 0.5em in Second Ink and raised as superscripts.
- **Chart Title** (640, wdth 118%, 0.3em tracking, uppercase, 1.05 to 1.3rem): the constellation name, and the title of the no-frame, no-match and first-run blocks. It is the atlas's chart title, not a kicker.
- **Headline** (650, 1.125rem, -0.01em): section titles ("In the field", "What to look at next") and sheet titles.
- **Title** (600, 0.875rem, Second Ink): group titles inside a section.
- **Body** (400, 1rem, 1.45, tabular figures): list entries and prose. Prose is capped at 60ch.
- **Body Quiet** (400, 0.9375rem): the fact lines (Clauses) and quiet notes.
- **Label** (600, 0.875rem, 0.12em, uppercase, Second Ink): the coordinate labels RA and Dec only. The epoch "J2000" uses 0.75rem at 0.18em.
- **Object Label** (italic 620, 13 chart units, cased in Frame Black): Messier IDs on the chart, and object IDs in lists (italic 600).
- **Edge Label** (520, wdth 82%, 11 chart units, Second Ink): graticule values in the paper margin.

### Named Rules
**The Italic Object Rule.** Deep-sky object identifiers are always italic, on the chart and in lists. Star names stay upright.

**The Tabular Figures Rule.** `tnum` is on for the whole body. Coordinates, magnitudes and counts line up in columns.

## Layout

One column on a phone. With a target set, the reading order is the guidance block, the chart header (where it points), the chart with its legend chips, then the field roster, field list, next targets and survey view. Without a target the field roster sits under the chart header, before the chart. From 64rem the main grid becomes two columns, `minmax(22rem, 30rem)` on the left for the header and lists and the chart on the right. The chart spans three rows and sticks at `top: 6rem` while the left column scrolls. With a target, guidance spans both columns above the title and chart. The page width follows the frame. `.page-width` caps at `min(90rem, 30rem + 2.5rem + (100dvh − 11rem) × frame aspect + 5rem)`, so a wide window holds the left column plus the chart at window height, centred. The chart is likewise capped at `(100dvh − 11rem) × aspect`.

Gutters are 1rem on phones, 1.25rem from 40rem and 2.5rem from 64rem. Vertical gaps are 1rem, 1.25rem and 2rem across those widths, with a 2.5rem column gap. The top bar is 3.5rem tall on phones and 4rem from 40rem. Sections stack at 1.5 to 2.5rem, and list rows have 10px of vertical padding over a 4-track grid (`1.5rem 3.25rem 1fr auto`: symbol, ID, name and detail, measure). The chart keeps a 30-unit paper margin round the frame for edge labels. Facts in a line are joined by centred dots that wrap only between facts, and no line starts or ends with a dot.

Breakpoints in use: 40rem (`sm`, wordmark and target label appear), 48rem (sheet centres), 64rem (`lg`, two columns).

## Elevation & Depth

The system is flat and uses no box-shadows. Depth is drawn the way a printed chart draws it: 1px ink rules, a doubled rule (two 1px boxes 3px apart) for the title block, hairline dividers, and tone steps from Cool Paper to Shaded Paper to Hairline. On the chart, labels and leaders lift off the photograph with a stroke casing in Frame Black (`paint-order: stroke`) rather than a shadow. Modal sheets sit over a translucent ink backdrop by day (`ink` at 42%) and black at 80% by night.

### Named Rules
**The Neatline Rule.** If something needs separation, rule it. Use 1px ink for a box, the doubled rule for the title block, and the hairline between rows. Never a shadow, never a tinted card.

## Shapes

Every box is square: buttons, chips, the switch and its thumb, inputs, the chart frame and the title block all have 0 radius and a 1px ink border. The exceptions are functional. The dialog sheet rounds its top corners at 12px as a bottom sheet on phones, and all four from 48rem where it centres. Skeleton bars round at 4px, and the focus outline at 3px. A dashed border means "not drawn", following the atlas convention: a legend chip whose layer is off, expected (unseen) star rings, and open-cluster symbols. Deep-sky symbols follow the atlas set: a square for a diffuse nebula, a dotted circle for an open cluster, a crossed circle for a globular, an ellipse for a galaxy, a rayed circle for a planetary nebula, and a crossed square for a supernova remnant. They are drawn at a 1.6 stroke. UI icons sit on a 20-unit grid with a 1.6 stroke, round caps, in currentColor, inline SVG only.

## Components

### Buttons
Ruled, square and quiet. The ink does the work.
- **Shape:** square (0), 1px ink border, at least 44px tall, 16px side padding, weight 560, optional 18px leading icon with a 0.5rem gap.
- **Primary:** an ink fill with paper text. Hover goes to Second Ink. At night it is a Night Rule fill with Night Ink text and a Night Ink border, and hover goes to Night Ground 2.
- **Default:** transparent with ink text. Hover fills Shaded Paper and press fills Hairline, in 160ms on the expo-out curve. Disabled is 45% opacity.
- **Quiet:** transparent border, used for "Close" in sheets.
- **Link:** ink text with a 1px underline offset 0.2em, weight 560.

### Chips
- **Style:** a ruled box, 38px tall, 10px side padding, 0.875rem at weight 520, with a 1px ink border on Cool Paper. Used for the top bar's frame and site controls, and for the chart legend's layer toggles with a symbol swatch and a count in Second Ink.
- **State:** an off toggle (`aria-pressed=false`) takes a dashed Second Ink border, Second Ink text, a transparent ground and its swatch at 35%. Hover fills Shaded Paper.

### Switch
- **Style:** a square 44 × 24px ruled track with a 16px square ink thumb. When on, the track fills Hairline and the thumb slides 20px in 220ms on the expo-out curve. It carries Night mode, labelled "Night" with a torch icon on phones, and Track in the guidance block.

### Cards / Containers
- **Chart header:** the doubled-rule title block. Its padding is 10px, 14px and 12px (top, sides, bottom) on phones, and 12px, 20px and 16px from 40rem. It holds the chart title and J2000 on one line, RA and Dec at display size, a hairline, then two lines of facts, the second led by a check.
- **Lists:** no cards. Rows are divided by hairlines inside a top and bottom hairline. The selected row fills Shaded Paper.
- **Corner Style:** square. **Shadow Strategy:** none (see Elevation & Depth).

### Inputs / Fields
- **Style:** a 1px ink box, 44px tall, with a transparent ground, 12px padding and a trailing unit (°, σ) in Second Ink. The label above is 0.875rem at weight 600 in Second Ink.
- **Focus:** a 2px ink outline offset 2px on the field box.

### Navigation
- **Top bar:** sticky, Cool Paper ground, a hairline bottom rule. It carries the reticle mark and "Finder" (weight 600), then the frame chip with a 28px thumbnail, the target-sheet button, the site chip, the Setup link, Stop while the mount may move, and the Night switch. On phones the wordmark, frame label and target button label are hidden to leave room for the controls.

### Sheets
- **Style:** a native `<dialog>`, Cool Paper ground, 1px ink border, up to 640px wide (448px for the site sheet). It rises from the bottom on phones and centres from 48rem. The header row has a headline title, a quiet Close button and a hairline below. It enters with a 280ms rise and fade on the expo-out curve. The built target sheet searches the catalog as typed and shows its ordered answers as hairline-divided rows: family symbol, italic catalog and Messier IDs, common names, kind, magnitude and pointability class. Picking or clearing saves the shared session before closing; a failed save keeps the sheet open with an ink error.

### Chart (signature)
The frame sits inside a 1.4-unit ink neatline with a 30-unit paper margin. The RA/Dec graticule is drawn over the frame in Graticule, animating on in 900ms. Ticks and condensed edge labels sit in the margin on all four sides. Matched stars get solid Frame Ink rings with upright names. Expected stars get dashed rings at 70%, and detected centroids get small rings at 55%. Messier objects get atlas symbols in their object colour with italic labels, and leaders run to labels set off in open sky. Compass and scale-bar furniture are in Frame Ink. Labels are laid out to avoid each other (`src/labels.ts`). A picked object thickens to 2.4. Marks ink in over 500ms after a 250 to 300ms delay. With reduced motion all animation is off.

### Guidance block — built
An ink-only ruled block leads the page when a target is set, before the doubled-rule title block. It names the target and kind, and prints the session's pointability class in Second Ink. Pointable guidance gives the whole-degree turn and tilt and the latest pointing's age, or asks for a solved frame. Down targets show the session's rising, transit and setting in device-local time to the minute; a target that never reaches the minimum altitude says so. The Sun guard refuses directions within 15° of the Sun. Without a site the block links to `/setup`. It uses no object colours, fill or shadow; night uses the existing red ink tokens.

### Target reticle — built
On the latest pointing's chart, the target's atlas family symbol sits at the supplied pixel coordinates inside four open square corner ticks in Frame Ink. Galaxy symbols retain the supplied axes and position angle. The italic object-colour label sits toward the interior of the frame. This is session furniture, outside the legend: the five chips and their counts stay the same, and hiding Deep sky leaves the target drawn. A missing or different `pointing_seq` suppresses the mark. Existing object and Frame Ink tokens collapse to red at night.

### GoTo — built
`GoToControl` sits inside the guidance block, under its directions, while mount telemetry is present. Its square 44px primary button uses the existing dim primary treatment at night. Only a connected mount with no running GoTo or second sync and no `goto_refusal` enables it. The refusal is the server planner's exact text, linked to the button and printed in Second Ink. The page does no motion planning. An ink status line shows Slewing, Solving, each numbered Correction, and On target with the final error rounded to 0.1″; failures and halts retain their reason. Requests refresh the session immediately, and mount sessions poll every 500ms, waiting for an outstanding read to finish.

### Stop — built
`StopButton` lives in the sticky top bar shared by the console and Setup. Mount motion or an active GoTo or second sync shows it, including settling and solving before moving telemetry arrives. With a connected mount, Track in `armed`, `tracking` or `keyhole_wait` also keeps Stop visible on every page, including Setup, even with both axes at rest; an absent or ended Track adds no Stop. It stays enabled through pending and failed requests and sends a bodyless `POST /api/stop`. The square ink button is at least 44px tall. No mount telemetry means no GoTo or Stop, preserving the phone's existing first screen.

With a mount, phone top-bar gaps tighten to 4px; below 380px the site chip yields to Stop, with the site still available in Setup. The bar fits at 320px without clipping Stop or the Night switch.

With mount telemetry present, both the target and frame sheets use nonmodal dialogs below the top bar (inset 4rem, layer 20 below the bar's 30). Native modal dialogs would make Stop inert even above their backdrop. These sheets leave the top bar interactive and close with Close or Escape; they never contain Stop. Errors use the existing ink tokens, red only at night.

### Track and tracking readout — built
`TrackControl` sits beside GoTo in the guidance block. Only a connected mount offers its labelled Track Switch, using the square 44 × 24px track and ink thumb above. It sends `POST /api/track` with `on`, then refreshes the shared session; the switch reflects the server's wanted state and turns off when tracking ends. An ink status names the tracked target, the wait for GoTo or the keyhole wait. Endings remain inside the labelled Track region: stopped by the observer, scope link lost, Sun up, target too low, or the server's GoTo reason. Refused requests stay readable as ink alerts.

The tracking readout uses tabular figures and Second Ink labels: Tracking error (RMS, last minute) rounded to 1″, Field rotation to 0.01°/min, and, with an imaging camera set, Longest exposure (1 px smear) to 0.1 s or the exact text “no limit”. Values wrap on phones. The page only formats the session's readout. No mount means no Track region or added height above the chart. All parts use existing red-only night tokens.

### Imaging camera — built
Setup's `ImagingCameraSection`, below Mount and pointing model, sets Sensor width and Sensor height in mm and Pixel pitch in µm. Its labelled transparent 44px fields have square 1px ink borders, trailing units in Second Ink and the existing 2px focus outline. The fields follow a two-column grid and retain unsaved drafts across session polls. Save imaging camera puts all three values in the shared session's `imaging_camera`; Forget imaging camera puts `null`. Successful saves restore the exposure readout when returning to the console. Each dimension must be finite, greater than 0 and at most 100 mm; pitch must be finite, greater than 0 and at most 50 µm. Invalid entries save nothing and print both allowed ranges in an ink alert, linked to the fields alongside the standing range help. Hairline rules separate the section, with the same red-only ink and ground at night.

### Mount and pointing model — built
Setup's `MountSection`, below Scope node and camera, uses hairline rules and ink text for four parts: the mount's Connected/Disconnected link, scope address, board and reported firmware; “base tilt 1.0° toward 130°” (magnitude to 0.1°, direction to whole degrees); RMS residual to 0.1″ and the sync count; Second sync and Reset night's model / Reset fixed terms. Missing board data says Board not reported, and an unfitted base tilt explains the two syncs at least 90° apart. Second sync sends `/api/model/second-sync`; the resets send `/api/model/reset` with `night` or `fixed`. Controls are square and at least 44px tall. Pending requests and motion disable model actions while top-bar Stop remains enabled. Successes and refusals remain readable alongside the controls, using the same red-only tokens at night.

### Edge arrow — built
An off-frame target uses the existing DirectionArrow, on the ray from the chart centre toward the supplied bearing (clockwise from up). It meets a neatline inset by 12 CSS pixels so the arrow stays in the frame, with an inward italic label giving the target ID and whole-degree distance. It shares the target family's object colour and the red-only night tokens, and is outside the legend like the reticle.

### Setup page — built
The `/setup` page edits the shared observing session: site latitude and longitude, typed or taken from the device, and minimum altitude from 0° to 80°. It uses labelled 44px ruled fields, ink links and buttons, hairline separators and a doubled-rule title block. Saving and clearing the site use the session hook; validation and failed requests stay readable in ink. The top bar's Setup link opens it, and its Back to console action returns to the chart. The same paper and ink tokens make it red on black at night.

### Live chart and preview — built
While the scope link is connected, live events feed the existing chart, its title block, stars and target guidance. Pointing sequence numbers keep a newer uploaded solve on screen until a newer live solve arrives. The five legend chips are unchanged. The occasional JPEG preview sits below the chart and its legend, inside `.photo` so the night edition passes it through the luminance-to-red filter. It keeps the last preview between photographs. Nothing is added above the phone's legend.

### Stale pointing — built
More than 5 s after a live pointing, the guidance block replaces its directions with “Pointing stale” and “Waiting for a live solve.” The chart title replaces its solved-time clause with “Pointing stale.” These replacements use existing lines and ink tokens. A 250 ms timer uses the console time supplied by session reads and live events; the next live solve restores guidance. Uploaded pointings keep their existing behaviour.

### Scope node and camera — built
Below the site and minimum altitude on Setup, an ink-only section shows Connected or Disconnected, the camera model and the latest star count. Session reads update link and camera information; live events update the count. Labelled 44px ruled fields hold the address, a password token, exposure in ms, gain and detection threshold in σ. The token starts empty on every device, says “A token is set” when paired, is omitted from a save when left empty, and clears after a successful save. Exposure and gain accept `auto` or a positive value; the threshold accepts 1–50 or `auto` for cedar's 8 σ. One Save scope node and camera action writes all entered fields. Drafts survive live events, and failures stay readable beside the fields. Stored exposure or gain outside the connected camera's range names the value and range and explains that the camera uses automatic while the stored value is kept.

The camera's last solved field is printed beside the loaded database's range. Within 1° of either end or outside it, an ink warning says a database built for the camera's field may be needed. After 30 s without a solve from frames with at least 10 stars, the warning says the camera's field may be outside that range and a matching database may be needed. An unsolved field is labelled “not solved yet.”

### Aim-point tap — built
Setup's Aim point section asks the observer to centre a star in the eyepiece and tap it on the live preview. The full-width ruled photograph uses `.photo` in the night edition. Each tap scales from the displayed image rectangle to that preview's full-frame pixels and sends its frame id; later frames without a preview do not change that coordinate system. The console snaps to a star. A refusal such as “no star is close enough to the tap,” or an expired frame, stays visible in ink and leaves the aim point unchanged. Unloaded or failed photographs accept no tap. The saved pixel coordinates are printed above the preview; Reset aim point immediately returns them to “frame centre,” even before a preview arrives. Moving the finder camera is the cue to reset.

## Do's and Don'ts

### Do:
- **Do** draw new chart content as atlas furniture: 1px or chart-unit strokes, currentColor, cased labels, edge labels in the paper margin.
- **Do** give every new colour a night value on the red channel no brighter than Night Ink (`#e00000`), and put every new raster or embedded view under the `.photo` red-only filter.
- **Do** separate with rules: a 1px ink box for controls, the doubled 3px-gap rule for a title block, hairlines between rows.
- **Do** set object IDs in italic and keep figures tabular.
- **Do** keep controls square and at least 44px (38px for chips) tall.
- **Do** use the expo-out curve (`cubic-bezier(0.16, 1, 0.3, 1)`) at 160 to 280ms for state changes, with every animation removed under reduced motion.

### Don't:
- **Don't** use navy glass cards, cyan glow or starfield wallpaper.
- **Don't** show a white or non-red pixel in night mode, including in images, previews, embedded views and the first paint.
- **Don't** use the nebula, cluster or galaxy colours on chrome, controls or text that is not an object.
- **Don't** use a box-shadow, or round a control's corners.
- **Don't** fill a night control with solid Night Ink.
