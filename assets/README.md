# The pictures

Three of them, in FAR Manager's own palette — the sixteen console colours the themes were
taken from, element for element (`crates/fcmd-ui/themes/far.toml`). Blue `#000080` panels,
light cyan `#00ffff` frames and text, a cyan `#008080` bar under the cursor and along the key
bar, yellow `#ffff00` for what is marked. Nothing here invents a colour the program does not
draw.

| File | What it is | Where it goes |
| --- | --- | --- |
| `logo.svg` | the animated one: the cursor walks the left panel while the right one counts off the systems | the top of `README.md` |
| `logo.png` | one frame of it, for anywhere that will not animate | fallbacks, slides |
| `banner.svg`, `banner.png` | 1280×640 | the repository's social preview (Settings → Social preview takes the PNG) |
| `icon.svg` | the program's icon, drawn to survive sixteen pixels | desktop entries, package metadata |
| `icon-16..256.png`, `fcmd.ico` | the same, rasterised | Windows, and anything that wants a PNG |

## What they say

Norton Commander gave the terminal its shape in 1986 and FAR kept its keys, but the thing
itself was never one thing: FAR is Windows, far2l is its Linux port, Midnight Commander is
Unix, Total Commander is Windows again, Double Commander is a third codebase. The same idea,
five times over, with different keys and different gaps on every system.

So the pictures say one program, every system: two panels in the old colours, and the three
systems ticked off beside them. The icon is the same sentence at sixteen pixels — two panels,
one cursor bar, one key bar.

## Why the animation is CSS and not a GIF

An SVG with a `<style>` block animates when GitHub shows it as an image, stays sharp at any
size, weighs six kilobytes, and can be read and edited as text in a pull request. It also
stops moving for anyone whose system asks it to: `prefers-reduced-motion` leaves the same
picture standing still.

## Why there is no image bot

Every picture here is either generated or the source of something generated: the SVGs in
`docs/shots/` are what `write_the_screenshots` writes, and the PNGs and the `.ico` come from
the SVGs beside them. An optimisation committed on top of either is overwritten the next time
they are made again, and until then what is in the repository is no longer what the generator
produces.

The three SVGs drawn by hand are the ones a bot could honestly touch, and one of them is
`logo.svg` — animated with a `<style>` block, and an SVG rather than a GIF precisely so that it
can be read and edited as text in a pull request. An automated rewrite is the one thing that
costs there. So `.imgbotconfig` ignores everything, and the pictures are made smaller, when
they need to be, by drawing them with fewer elements.

## Making the rasters again

The SVGs are the source; the PNGs and the `.ico` come from them. Chromium draws the SVG
exactly as a browser will, which is the point of using it rather than a converter:

```sh
node tools/render-assets.cjs          # needs playwright and ImageMagick's `convert`
```
