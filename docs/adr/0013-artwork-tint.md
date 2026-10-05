# 13. Deterministic artwork tint derivation

Date: 2026-10-04
Status: proposed implementation ruling for WP-238 under the approved plan.

## Context

[WP-238](../plan/work-packages.md#wp-238-tint-surfaces-from-an-artwork-colour-added-for-the-client-plan)
shares [design-language section 5](../ui/design-language.md#5-how-artwork-drives-colour)
across clients. Its interface sketch leaves the canvas and token roles
implicit. The design specifies chroma reduction without a numerical search
method, and a placeholder ID hash without a hash convention.

## Decisions

- Accept the client canvas as an sRGB number triple. Normal text requires
  4.5:1; controls or large text require 3:1. A closed token-role enum carries
  those floors beside each client colour. Token values stay in clients.
- Use WP-037's near-grey threshold of 0.04 on the **base candidate** before
  applying theme caps. It describes almost-grey artwork, not the derived
  tint: light-theme gamut reduction may produce chroma below 0.04.
- Preserve lightness and hue while searching for the largest in-gamut
  chroma with 32 bisections. This bounds work and chroma uncertainty below
  `0.07 / 2^32`. Check linear sRGB channels before encoding them; do not
  clip out-of-gamut RGB channels.
- Start at L=0.26,C<=0.07 for Dark and OLED, and L=0.95,C<=0.04 for Light.
  Try steps of exactly 0.01 toward the client's canvas lightness, at most
  101 candidates. Recompute gamut and contrast for each candidate. Stop
  before crossing the canvas; do not reverse direction when the supplied
  canvas is inconsistent with the theme. Return no tint for invalid input,
  near-grey artwork, high contrast, or an unsatisfied contrast requirement.
- Return the sRGB surface and actual OKLCH coordinates. Header and player
  blends remain client work; this package adds no gradient rendering.
- Hash the canonical PublicId spelling with FNV-1a 64 and take the result
  modulo 360 for the placeholder's whole-degree hue. Include the kind
  prefix, use explicit wrapping multiplication, and never use Rust's
  process-dependent default hasher. This is a visual hash, not a security
  primitive.

## Consequences

The interface has explicit canvas and role inputs beyond the plan's sketch.
Malformed floating-point values produce `None`; a hostile candidate cannot
change foreground tokens or bypass contrast validation. The colours use
the inverse OKLab matrices corresponding to WP-037 and the WCAG 2 sRGB
transfer curve and relative luminance. Literal examples, a one-degree hue
sweep, and independent WCAG-oracle properties verify the result.
