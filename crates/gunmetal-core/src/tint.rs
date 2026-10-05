//! Artwork tint surfaces shared by clients (WP-238).

use crate::id::PublicId;
use crate::imagedata::Swatch;

/// OKLCH numbers: unit lightness, chroma, and hue in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Lightness in `0..=1`.
    pub lightness: f64,
    /// Chroma in `0..=0.5`.
    pub chroma: f64,
    /// Hue in `0..360`.
    pub hue: f64,
}

/// Nonlinear sRGB channels in `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Srgb {
    /// Red channel.
    pub red: f64,
    /// Green channel.
    pub green: f64,
    /// Blue channel.
    pub blue: f64,
}

/// Theme selected by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Dark steel surfaces.
    Dark,
    /// Pale steel surfaces.
    Light,
    /// Dark tint with a black canvas.
    Oled,
    /// Artwork tint is disabled.
    HighContrast,
}

/// WCAG AA role of a token placed on the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContrastRole {
    /// Normal-sized text: 4.5:1.
    Text,
    /// Controls, graphics, focus indicators, or large text: 3:1.
    ControlOrLargeText,
}

/// Client token colour and the contrast floor its use requires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnSurface {
    /// Token colour.
    pub colour: Srgb,
    /// Token role.
    pub role: ContrastRole,
}

/// The colour shared by the header wash and player backdrop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surfaces {
    /// Gamut-reduced and contrast-checked sRGB surface.
    pub surface: Srgb,
    /// Actual OKLCH coordinates used to produce `surface`.
    pub oklch: Oklch,
}

/// Stable placeholder hue, in whole degrees below 360.
pub type Hue = u16;

/// Derives a surface from an artwork candidate and client token colours.
///
/// Returns `None` for invalid numeric input, base chroma below 0.04,
/// high contrast, an inconsistent canvas direction, or unsatisfied contrast.
/// Gamut reduction preserves hue and lightness; the client owns gradients.
#[must_use]
pub fn surfaces(
    base: Oklch,
    theme: Theme,
    canvas: Srgb,
    on_surface: &[OnSurface],
) -> Option<Surfaces> {
    if !(0.0..=1.0).contains(&base.lightness)
        || !(0.04..=0.5).contains(&base.chroma)
        || !(0.0..360.0).contains(&base.hue)
        || !valid_srgb(canvas)
        || on_surface.iter().any(|token| !valid_srgb(token.colour))
    {
        return None;
    }
    let (initial, cap, direction) = match theme {
        Theme::Dark | Theme::Oled => (0.26, 0.07, -1.0),
        Theme::Light => (0.95, 0.04, 1.0),
        Theme::HighContrast => return None,
    };
    let target = canvas_lightness(canvas);
    for step in 0_u8..=100 {
        let lightness = initial + direction * 0.01 * f64::from(step);
        if !(0.0..=1.0).contains(&lightness) || direction * (target - lightness) < 0.0 {
            break;
        }
        let result = in_gamut(Oklch {
            lightness,
            chroma: base.chroma.min(cap),
            hue: base.hue,
        });
        let background = luminance(result.surface);
        if on_surface.iter().all(|token| {
            let foreground = luminance(token.colour);
            let ratio = (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05);
            let floor = match token.role {
                ContrastRole::Text => 4.5,
                ContrastRole::ControlOrLargeText => 3.0,
            };
            ratio >= floor
        }) {
            return Some(result);
        }
    }
    None
}

/// Derives the missing-art placeholder hue from an item's canonical ID.
#[must_use]
pub fn placeholder_hue(id: &PublicId) -> Hue {
    let hash = id
        .to_string()
        .bytes()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(1_099_511_628_211)
        });
    // Modulo 360 fits in the low two bytes on every target.
    let [low, high, ..] = hash.rem_euclid(360).to_le_bytes();
    u16::from_le_bytes([low, high])
}

impl From<Swatch> for Oklch {
    fn from(swatch: Swatch) -> Self {
        Self {
            lightness: f64::from(swatch.lightness) / 1000.0,
            chroma: f64::from(swatch.chroma) / 1000.0,
            hue: f64::from(swatch.hue),
        }
    }
}

fn valid_srgb(colour: Srgb) -> bool {
    [colour.red, colour.green, colour.blue]
        .into_iter()
        .all(|channel| (0.0..=1.0).contains(&channel))
}

fn linear(channel: f64) -> f64 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn encoded(channel: f64) -> f64 {
    if channel <= 0.003_130_8 {
        12.92 * channel
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

fn luminance(colour: Srgb) -> f64 {
    0.2126 * linear(colour.red) + 0.7152 * linear(colour.green) + 0.0722 * linear(colour.blue)
}

fn canvas_lightness(colour: Srgb) -> f64 {
    let red = linear(colour.red);
    let green = linear(colour.green);
    let blue = linear(colour.blue);
    let long = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
    let medium = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
    let short = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();
    0.210_454_255_3 * long + 0.793_617_785_0 * medium - 0.004_072_046_8 * short
}

/// Inverse of WP-037's `OKLab` matrices, before the sRGB transfer curve.
fn rgb_linear(colour: Oklch) -> Srgb {
    let angle = colour.hue.to_radians();
    let a = colour.chroma * angle.cos();
    let b = colour.chroma * angle.sin();
    let long = (colour.lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let medium = (colour.lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let short = (colour.lightness - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    Srgb {
        red: 4.076_741_662_1 * long - 3.307_711_591_3 * medium + 0.230_969_929_2 * short,
        green: -1.268_438_004_6 * long + 2.609_757_401_1 * medium - 0.341_319_396_5 * short,
        blue: -0.004_196_086_3 * long - 0.703_418_614_7 * medium + 1.707_614_701 * short,
    }
}

fn in_gamut(mut colour: Oklch) -> Surfaces {
    let mut rgb = rgb_linear(colour);
    if !valid_srgb(rgb) {
        let mut low = 0.0;
        let mut high = colour.chroma;
        for _ in 0..32 {
            let chroma = f64::midpoint(low, high);
            let trial = rgb_linear(Oklch { chroma, ..colour });
            if valid_srgb(trial) {
                low = chroma;
            } else {
                high = chroma;
            }
        }
        colour.chroma = low;
        rgb = rgb_linear(colour);
    }
    Surfaces {
        surface: Srgb {
            red: encoded(rgb.red),
            green: encoded(rgb.green),
            blue: encoded(rgb.blue),
        },
        oklch: colour,
    }
}

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "literal design coordinates compare exactly"
)]
mod tests {
    use super::*;
    use crate::id::IdKind;
    use proptest::prelude::*;

    const BLACK: Srgb = Srgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    const WHITE: Srgb = Srgb {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
    };
    const BASE: Oklch = Oklch {
        lightness: 0.7,
        chroma: 0.04,
        hue: 0.0,
    };

    fn near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    fn colour_near(actual: Srgb, expected: [f64; 3]) {
        for (a, e) in [actual.red, actual.green, actual.blue]
            .into_iter()
            .zip(expected)
        {
            near(a, e);
        }
    }

    // Independent WCAG 2 oracle, never calls a production colour helper.
    fn luminance(colour: Srgb) -> f64 {
        [colour.red, colour.green, colour.blue]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(v, weight)| {
                weight
                    * if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }

    fn contrast(a: Srgb, b: Srgb) -> f64 {
        let a = luminance(a);
        let b = luminance(b);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn grey(value: f64) -> Srgb {
        Srgb {
            red: value,
            green: value,
            blue: value,
        }
    }

    fn hex([r, g, b]: [u8; 3]) -> Srgb {
        Srgb {
            red: f64::from(r) / 255.0,
            green: f64::from(g) / 255.0,
            blue: f64::from(b) / 255.0,
        }
    }

    #[test]
    fn dark_uses_fixed_lightness_and_preserves_uncapped_candidate_chroma() {
        let result = surfaces(BASE, Theme::Dark, BLACK, &[]).unwrap();
        assert_eq!(
            result.oklch,
            Oklch {
                lightness: 0.26,
                ..BASE
            }
        );
        // Inverse OKLab matrix, independently evaluated at L=.26,C=.04,h=0.
        colour_near(
            result.surface,
            [
                0.203_229_323_273_647_46,
                0.107_881_661_241_037_35,
                0.137_846_287_478_020_22,
            ],
        );
    }

    #[test]
    fn oled_uses_dark_cap_and_ignores_artwork_lightness() {
        let base = Oklch {
            lightness: 0.0,
            chroma: 0.5,
            hue: 0.0,
        };
        let result = surfaces(base, Theme::Oled, BLACK, &[]).unwrap();
        assert_eq!(
            result.oklch,
            Oklch {
                lightness: 0.26,
                chroma: 0.07,
                hue: 0.0
            }
        );
        colour_near(
            result.surface,
            [
                0.241_602_679_247_515_55,
                0.072_507_707_881_161_98,
                0.135_573_940_891_843_6,
            ],
        );
    }

    #[test]
    fn light_caps_chroma_at_four_hundredths() {
        let base = Oklch {
            chroma: 0.5,
            hue: 90.0,
            ..BASE
        };
        let result = surfaces(base, Theme::Light, WHITE, &[]).unwrap();
        assert_eq!(
            result.oklch,
            Oklch {
                lightness: 0.95,
                chroma: 0.04,
                hue: 90.0
            }
        );
        colour_near(
            result.surface,
            [
                0.975_766_548_344_616_6,
                0.933_168_815_292_831_4,
                0.818_716_955_851_374_6,
            ],
        );
    }

    #[test]
    fn gamut_reduction_preserves_lightness_and_hue_instead_of_clipping_rgb() {
        let result = surfaces(
            Oklch {
                chroma: 0.07,
                hue: 180.0,
                ..BASE
            },
            Theme::Dark,
            BLACK,
            &[],
        )
        .unwrap();
        near(result.oklch.chroma, 0.047_171_778_751_727_5);
        assert_eq!(result.oklch.lightness, 0.26);
        assert_eq!(result.oklch.hue, 180.0);
        colour_near(
            result.surface,
            [0.0, 0.169_701_732_112_226_58, 0.144_430_362_360_627_98],
        );
    }

    #[test]
    fn base_floor_is_inclusive_and_does_not_apply_to_gamut_reduced_light_tints() {
        for c in [0.0, 0.02, 0.039_999_999_999] {
            assert_eq!(
                surfaces(Oklch { chroma: c, ..BASE }, Theme::Dark, BLACK, &[]),
                None
            );
        }
        let result = surfaces(BASE, Theme::Light, WHITE, &[]).unwrap();
        assert_eq!(result.oklch.lightness, 0.95);
        near(result.oklch.chroma, 0.026_856_563_109_904_533);
        colour_near(
            result.surface,
            [
                0.999_999_999_981_085_5,
                0.907_657_970_174_959_4,
                0.931_674_682_090_440_2,
            ],
        );
    }

    #[test]
    fn contrast_failure_steps_dark_lightness_once_and_checks_all_tokens() {
        let tokens = [
            OnSurface {
                colour: WHITE,
                role: ContrastRole::Text,
            },
            OnSurface {
                colour: grey(0.53),
                role: ContrastRole::Text,
            },
        ];
        let result = surfaces(BASE, Theme::Dark, BLACK, &tokens).unwrap();
        near(result.oklch.lightness, 0.25);
        colour_near(
            result.surface,
            [
                0.192_948_015_542_496_71,
                0.098_585_495_534_011_41,
                0.128_504_539_151_391_93,
            ],
        );
        near(contrast(result.surface, grey(0.53)), 4.528_115_633_796_113);
    }

    #[test]
    fn control_floor_steps_light_toward_canvas_but_text_role_requires_more() {
        let base = Oklch { hue: 90.0, ..BASE };
        let token = OnSurface {
            colour: grey(0.54),
            role: ContrastRole::ControlOrLargeText,
        };
        let result = surfaces(base, Theme::Light, WHITE, &[token]).unwrap();
        near(result.oklch.lightness, 0.96);
        colour_near(
            result.surface,
            [
                0.988_911_482_684_722,
                0.946_215_659_570_276_5,
                0.831_489_333_883_134_6,
            ],
        );
        near(
            contrast(result.surface, token.colour),
            3.088_366_225_484_225,
        );
        assert_eq!(
            surfaces(
                base,
                Theme::Light,
                WHITE,
                &[OnSurface {
                    role: ContrastRole::Text,
                    ..token
                }]
            ),
            None
        );
    }

    #[test]
    fn stops_at_canvas_instead_of_stepping_past_it_or_reversing_direction() {
        let token = OnSurface {
            colour: BLACK,
            role: ContrastRole::Text,
        };
        assert_eq!(surfaces(BASE, Theme::Dark, BLACK, &[token]), None);
        assert_eq!(surfaces(BASE, Theme::Dark, grey(0.5), &[token]), None);
        assert_eq!(
            surfaces(
                BASE,
                Theme::Light,
                BLACK,
                &[OnSurface {
                    colour: WHITE,
                    role: ContrastRole::Text
                }]
            ),
            None
        );
        assert_eq!(surfaces(BASE, Theme::HighContrast, BLACK, &[]), None);
        // A canvas on the wrong side of the starting lightness refuses even
        // with no token to satisfy: lightness 0.9351 under Light's 0.95, and
        // 0.2648 over Dark's 0.26.
        assert_eq!(
            surfaces(BASE, Theme::Light, hex([230, 234, 238]), &[]),
            None
        );
        assert_eq!(surfaces(BASE, Theme::Dark, hex([31, 38, 45]), &[]), None);
    }

    #[test]
    fn stepping_stops_at_a_canvas_that_lies_before_the_first_passing_step() {
        let token = OnSurface {
            colour: grey(0.52),
            role: ContrastRole::Text,
        };
        // Against a black canvas the third step, lightness 0.23, is the
        // first that reaches 4.5:1.
        let result = surfaces(BASE, Theme::Dark, BLACK, &[token]).unwrap();
        near(result.oklch.lightness, 0.23);
        colour_near(
            result.surface,
            [
                0.172_575_959_656_687_12,
                0.080_235_981_922_731_63,
                0.110_106_479_901_619_06,
            ],
        );
        near(
            contrast(result.surface, token.colour),
            4.612_077_414_455_852,
        );
        // This grey canvas has lightness 0.2376, between the second and
        // third steps, so the third step would cross it.
        assert_eq!(surfaces(BASE, Theme::Dark, grey(0.12), &[token]), None);
    }

    #[test]
    fn a_surface_may_sit_exactly_at_the_canvas_lightness() {
        // This grey reaches 4.5:1 only against a black surface: 4.50004 at
        // lightness 0, and 4.49996 one step lighter.
        let token = OnSurface {
            colour: grey(0.455_333),
            role: ContrastRole::Text,
        };
        assert_eq!(
            surfaces(BASE, Theme::Dark, BLACK, &[token]),
            Some(Surfaces {
                surface: BLACK,
                oklch: Oklch {
                    lightness: 0.0,
                    chroma: 0.0,
                    hue: 0.0
                },
            })
        );
        near(contrast(BLACK, token.colour), 4.500_038_794_375_35);
    }

    #[test]
    fn canvas_lightness_is_the_oklab_lightness_of_chromatic_colours() {
        // Ottosson's sRGB to OKLab matrices, evaluated independently.
        for (colour, expected) in [
            (
                Srgb {
                    red: 0.0,
                    green: 0.0,
                    blue: 1.0,
                },
                0.452_013_718_385_342_8,
            ),
            (
                Srgb {
                    red: 0.2,
                    green: 0.5,
                    blue: 0.9,
                },
                0.604_309_296_227_169,
            ),
            (
                Srgb {
                    red: 0.9,
                    green: 0.6,
                    blue: 0.1,
                },
                0.741_572_686_146_229_8,
            ),
        ] {
            near(canvas_lightness(colour), expected);
        }
    }

    #[test]
    fn rejects_nonfinite_and_out_of_range_inputs_without_sanitising_them() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.001] {
            for base in [
                Oklch {
                    lightness: value,
                    ..BASE
                },
                Oklch {
                    chroma: value,
                    ..BASE
                },
                Oklch { hue: value, ..BASE },
            ] {
                assert_eq!(surfaces(base, Theme::Dark, BLACK, &[]), None);
            }
        }
        for base in [
            Oklch {
                lightness: 1.001,
                ..BASE
            },
            Oklch {
                chroma: 0.501,
                ..BASE
            },
            Oklch { hue: 360.0, ..BASE },
        ] {
            assert_eq!(surfaces(base, Theme::Dark, BLACK, &[]), None);
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.001, 1.001] {
            for colour in [
                Srgb {
                    red: value,
                    ..BLACK
                },
                Srgb {
                    green: value,
                    ..BLACK
                },
                Srgb {
                    blue: value,
                    ..BLACK
                },
            ] {
                assert_eq!(surfaces(BASE, Theme::Dark, colour, &[]), None);
                assert_eq!(
                    surfaces(
                        BASE,
                        Theme::Dark,
                        BLACK,
                        &[OnSurface {
                            colour,
                            role: ContrastRole::Text
                        }]
                    ),
                    None
                );
            }
        }
    }

    #[test]
    fn converts_merged_palette_thousandths_without_using_stored_contrasts() {
        assert_eq!(
            Oklch::from(Swatch {
                lightness: 731,
                chroma: 142,
                hue: 359,
                contrast_black: 123,
                contrast_white: 456
            }),
            Oklch {
                lightness: 0.731,
                chroma: 0.142,
                hue: 359.0
            }
        );
    }

    #[test]
    fn placeholder_hash_is_stable_and_includes_kind_and_the_final_id_symbol() {
        for (text, kind, expected) in [
            ("alb_00000000000000000000000000", IdKind::Album, 75),
            ("alb_00000000000000000000000001", IdKind::Album, 344),
            ("art_00000000000000000000000000", IdKind::Artist, 135),
        ] {
            let id = PublicId::parse(text, kind).unwrap();
            assert_eq!(placeholder_hue(&id), expected);
            assert_eq!(placeholder_hue(&id), expected);
        }
    }

    #[test]
    fn every_degree_meets_the_design_table_floors_at_zero_half_and_full_caps() {
        for (theme, start, cap, canvas, colours) in [
            (
                Theme::Dark,
                0.26,
                0.07,
                hex([15, 19, 23]),
                [
                    [233, 238, 242],
                    [176, 187, 197],
                    [143, 155, 167],
                    [212, 149, 47],
                    [224, 173, 85],
                    [243, 205, 119],
                    [107, 119, 131],
                ],
            ),
            (
                Theme::Light,
                0.95,
                0.04,
                hex([243, 245, 247]),
                [
                    [17, 22, 27],
                    [58, 69, 80],
                    [86, 98, 110],
                    [184, 120, 26],
                    [133, 85, 15],
                    [122, 76, 12],
                    [122, 134, 147],
                ],
            ),
        ] {
            let tokens: Vec<_> = colours
                .into_iter()
                .zip([
                    ContrastRole::Text,
                    ContrastRole::Text,
                    ContrastRole::Text,
                    ContrastRole::ControlOrLargeText,
                    ContrastRole::Text,
                    ContrastRole::ControlOrLargeText,
                    ContrastRole::ControlOrLargeText,
                ])
                .map(|(rgb, role)| OnSurface {
                    colour: hex(rgb),
                    role,
                })
                .collect();
            let meets_every_floor = |surface: Srgb| {
                tokens.iter().all(|token| {
                    let floor = match token.role {
                        ContrastRole::Text => 4.5,
                        ContrastRole::ControlOrLargeText => 3.0,
                    };
                    contrast(surface, token.colour) >= floor
                })
            };
            for hue in (0..360).map(f64::from) {
                // The table's three rows: the band at its fixed lightness
                // with no chroma, half the cap and the whole cap.
                for chroma in [0.0, cap / 2.0, cap] {
                    let band = in_gamut(Oklch {
                        lightness: start,
                        chroma,
                        hue,
                    });
                    assert!(
                        meets_every_floor(band.surface),
                        "{theme:?}, {hue}, {chroma}"
                    );
                }
                // Through the public function, from the floor to far past
                // the cap: a tint, and no step away from the start.
                for chroma in [0.04, 0.055, 0.07, 0.5] {
                    let result = surfaces(
                        Oklch {
                            chroma,
                            hue,
                            ..BASE
                        },
                        theme,
                        canvas,
                        &tokens,
                    )
                    .unwrap();
                    assert_eq!(result.oklch.lightness, start, "{theme:?}, {hue}, {chroma}");
                    assert!(
                        meets_every_floor(result.surface),
                        "{theme:?}, {hue}, {chroma}"
                    );
                }
                // Just under the floor there is no tint.
                assert_eq!(
                    surfaces(
                        Oklch {
                            chroma: 0.039,
                            hue,
                            ..BASE
                        },
                        theme,
                        canvas,
                        &tokens
                    ),
                    None
                );
            }
        }
    }

    proptest! {
        #[test]
        fn valid_results_are_deterministic_in_gamut_and_meet_every_floor(l in 0.0..=1.0, c in 0.0..=0.5, h in 0.0..360.0, light in any::<bool>(), r in 0.0..=1.0, g in 0.0..=1.0, b in 0.0..=1.0, text in any::<bool>()) {
            let base = Oklch { lightness: l, chroma: c, hue: h };
            let theme = if light { Theme::Light } else { Theme::Dark };
            let canvas = if light { WHITE } else { BLACK };
            let token = OnSurface { colour: Srgb { red: r, green: g, blue: b }, role: if text { ContrastRole::Text } else { ContrastRole::ControlOrLargeText } };
            let result = surfaces(base, theme, canvas, &[token]);
            prop_assert_eq!(result, surfaces(base, theme, canvas, &[token]));
            if let Some(result) = result {
                for channel in [result.surface.red, result.surface.green, result.surface.blue] { prop_assert!((0.0..=1.0).contains(&channel)); }
                prop_assert!(result.oklch.chroma <= c.min(if light { 0.04 } else { 0.07 }), "theme chroma cap exceeded");
                prop_assert_eq!(result.oklch.hue, h);
                prop_assert!(contrast(result.surface, token.colour) >= if text { 4.5 } else { 3.0 }, "token contrast floor failed");
            }
        }
    }
}
