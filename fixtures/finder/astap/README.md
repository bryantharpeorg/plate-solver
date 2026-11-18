# ASTAP's real output, for spec 032

The owner recorded these on 2026-11-18 for spec 032's US1-S7, as its plan.md
§ Capturing ASTAP's output says: ASTAP's command-line program, run on one real picture
in each of the five cases US1-S7 replays. No test runs ASTAP. US1-S7 reads these files
through `captures.json` and replays each case through the stub program.

## The program and the database

- **ASTAP**: Ubuntu's `astap-cli` package, version 2025.10.08-1, whose `astap_cli`
  prints `ASTAP solver version CLI-2025.09.29`. `captures.json` gives that version.
- **The database**: D50, from the `d50` package, version 1.0 (Gaia DR3 stars), named
  with `-D d50`. The package puts it in `/opt/astap`, and Ubuntu's `astap_cli` looks in
  `/usr/share/astap/data/` unless told otherwise. So every case that needs the database
  passes `-d /opt/astap`. A first run without it ended every case but `-analyse` with
  `ERROR=No star database found.`.

## The files

All six are ASTAP's own, copied byte for byte.

- **`solved.ini`**: the solved case. `PLTSOLVD=T`, the eight numbers, `CDELT1/2`,
  `CROTA1/2` and `CMDLINE`, as bare `KEY=value` lines with no `//` note. A positive
  number follows a space (`CRPIX1= 1.1885000000000000E+003`). `CRPIX` is (1188.5,
  893.5), the centre of 2376 × 1786. The `CD` determinant is positive: rows in the
  file's own order, first row the top, an unmirrored sky.
- **`solved.wcs`**: the solved case's `.wcs`. It is a FITS header written as text: one
  card a line, the short `COMMENT` cards not padded to 80 characters, and the whole not
  padded to 2880 bytes. Its numbers equal `solved.ini`'s to the twelve places it
  writes. Nothing reads it.
- **`not-solved.ini`**: a hint 12 h of right ascension away with a radius of 1°. An
  empty line, then `PLTSOLVD=F` and `CMDLINE`, and no `ERROR` line. ASTAP printed
  `No solution found!  :(` on standard output.
- **`too-few-stars.ini`**: a 64 × 48 picture of one level, with no star. An empty line,
  `PLTSOLVD=F`, `CMDLINE`, `ERROR=Not enough stars.` and
  `WARNING=Warning, small image dimensions!! `, whose last character is a space.
- **`no-database.ini`**: `-d` naming a folder that does not exist. An empty line,
  `PLTSOLVD=F`, `CMDLINE` and `ERROR=No star database found.`. ASTAP printed where it
  had looked on standard output.
- **`analyse.stdout`**: standard output of `-analyse 30`: `HFD_MEDIAN=5.3` and
  `STARS=982`, one a line.
- **`captures.json`**: what US1-S7 reads: ASTAP's version, the database, the picture's
  size, each case's exit code and files, the three stars of the solved case, and the
  two numbers of the star measure.

ASTAP exited 0 for the solved case and for `-analyse`, and 1 for each of the three
failures. Each `CMDLINE` names only a temporary folder and `/opt/astap`.

## The picture

A cutout of the Digitized Sky Survey's second red survey (DSS2 red), fetched from
STScI's archive, MAST: POSS-II F, plate A0X3, taken with the Oschin Schmidt Telescope of
Caltech's Palomar Observatory in 1993. It is centred on M13 (16 41 41.24 +36 27 35.5,
J2000), 40′ wide and 30′ high, 2376 × 1786 samples of 16 bits, about 1.01″ a pixel.

ASTAP was given a prepared copy, written the way spec 032's `prepare` writes one: the
same samples, its rows in reverse order so that the first row in the file is the top
(north up, east left), with only the structural cards (`SIMPLE`, `BITPIX`, `NAXIS`,
`NAXIS1`, `NAXIS2`, `BZERO` 32768, `BSCALE` 1) and no WCS, plate or copyright card. The
picture itself is not committed.

## The three stars

The solved case's three stars, for US1-S7. Each `x` and `y` is the column and row of its
brightest sample in the prepared copy, counted from the first sample in the file, plus
0.5. Each place is the star's Gaia DR3 place, propagated to epoch 2025.0 (ICRS), the
epoch of ASTAP's star databases; it does not come from ASTAP.

| Gaia DR3 | G | Proper motion | x | y | `ra_deg` | `dec_deg` |
|---|---|---|---|---|---|---|
| 1328063227191564160 | 17.04 | 4.75 mas/yr | 243.5 | 444.5 | 250.747793 | 36.590747 |
| 1328109097442796288 | 17.28 | 5.79 mas/yr | 1688.5 | 517.5 | 250.244137 | 36.561196 |
| 1328052202019924608 | 17.07 | 4.14 mas/yr | 1050.5 | 1432.5 | 250.474140 | 36.309779 |

They do not lie in one line. Each is an unsaturated single peak, and no other Gaia
source lies within 5″ of it. Their proper motions are all under 6 mas a year, so the
1993 plate and the 2025 places differ by under 0.2″. `solved.ini` puts them 0.36, 0.14
and 0.56 pixels from those places, a FITS pixel being `x + 0.5`.

## How they were made

With the capture script of spec 032's plan.md, from a folder holding the prepared copy:
RA 16.694789 h, south polar distance 126.459861° (declination + 90°), field height
0.50°:

```bash
./capture-astap.sh m13-prepared.fits 16.694789 126.459861 0.50 out
```

The script copies the picture to a temporary folder as `frame.fits`, writes the blank
64 × 48 picture beside it as `blank.fits`, and runs, there:

```text
astap_cli -f frame.fits -o solved        -ra 16.694789 -spd 126.459861 -r 3 -fov 0.50 -z 1 -D d50 -d /opt/astap
astap_cli -f frame.fits -o not-solved    -ra 4.694789  -spd 126.459861 -r 1 -fov 0.50 -z 1 -D d50 -d /opt/astap
astap_cli -f blank.fits -o too-few-stars -ra 16.694789 -spd 126.459861 -r 1 -fov 0.50 -z 1 -D d50 -d /opt/astap
astap_cli -f frame.fits -o no-database   -ra 16.694789 -spd 126.459861 -r 1 -fov 0.50 -z 1 -D d50 -d nowhere
astap_cli -f frame.fits -analyse 30
```

The script gave every path but `/opt/astap` as an absolute one inside the temporary
folder, as the `CMDLINE` lines show; `nowhere` was never made.

## Credit and terms

The picture is from the Digitized Sky Survey, through MAST at the Space Telescope Science
Institute. STScI's terms (https://archive.stsci.edu/dss/copyright.html): the DSS images
and products derived from them may be used freely for scientific, educational and
non-profit purposes, under the Creative Commons Attribution-NonCommercial-ShareAlike 4.0
licence (CC BY-NC-SA 4.0, https://creativecommons.org/licenses/by-nc-sa/4.0/), which
forbids their use for profit. The files here hold no sample of the picture, but they are
ASTAP's answers about it, so the same terms apply to them.

The plate's copyright, as STScI states it:

> The compressed files of the "Palomar Observatory - Space Telescope Science Institute
> Digital Sky Survey" of the northern sky, based on scans of the Second Palomar Sky
> Survey are copyright (c) 1993-2003 by the California Institute of Technology and are
> distributed herein by agreement.

The acknowledgements STScI asks for, from MAST's Mission Acknowledgements page
(https://archive.stsci.edu/publishing/mission-acknowledgements, DSS), to which its
copyright page's Data Attributions link leads:

> The Digitized Sky Survey was produced at the Space Telescope Science Institute under
> U.S. Government grant NAG W–2166. The images of these surveys are based on
> photographic data obtained using the Oschin Schmidt Telescope on Palomar Mountain and
> the UK Schmidt Telescope. The plates were processed into the present compressed
> digital form with the permission of these institutions.

> The Second Palomar Observatory Sky Survey (POSS-II) was made by the California
> Institute of Technology with funds from the National Science Foundation, NASA, the
> National Geographic Society, the Sloan Foundation, the Samuel Oschin Foundation, and
> the Eastman Kodak Corporation. The Oschin Schmidt Telescope is operated by the
> California Institute of Technology and Palomar Observatory.

The three stars' places and the D50 database come from Gaia DR3: ESA/Gaia/DPAC.
