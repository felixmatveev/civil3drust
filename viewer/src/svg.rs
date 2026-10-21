//! SVG output of the same drawing the window shows (`--svg`).

use crate::view::{Canvas, Rgba, S2};
use std::fmt::Write;

pub struct Svg {
    body: String,
    w: f32,
    h: f32,
    bg: Rgba,
}

impl Svg {
    pub fn new(w: f32, h: f32, bg: Rgba) -> Self {
        Svg {
            body: String::new(),
            w,
            h,
            bg,
        }
    }

    pub fn finish(self) -> String {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\n\
             <rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>\n{body}</svg>\n",
            w = self.w,
            h = self.h,
            bg = css(self.bg),
            body = self.body
        )
    }
}

fn css(c: Rgba) -> String {
    format!("rgba({},{},{},{:.3})", c.0, c.1, c.2, c.3 as f32 / 255.0)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

impl Canvas for Svg {
    fn polyline(&mut self, pts: &[S2], closed: bool, color: Rgba, width: f32) {
        let tag = if closed { "polygon" } else { "polyline" };
        let mut p = String::new();
        for s in pts {
            let _ = write!(p, "{:.2},{:.2} ", s[0], s[1]);
        }
        let _ = writeln!(
            self.body,
            "<{tag} points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{width}\" stroke-linejoin=\"round\"/>",
            p.trim_end(),
            css(color)
        );
    }

    fn marker(&mut self, at: S2, color: Rgba, size: f32) {
        let _ = writeln!(
            self.body,
            "<circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{size}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.3\"/>",
            at[0],
            at[1],
            css(color)
        );
    }

    fn text(&mut self, at: S2, angle: f32, size: f32, text: &str, color: Rgba) {
        let _ = writeln!(
            self.body,
            "<text x=\"{x:.2}\" y=\"{y:.2}\" font-family=\"sans-serif\" font-size=\"{size:.2}\" fill=\"{c}\" transform=\"rotate({a:.3} {x:.2} {y:.2})\">{t}</text>",
            x = at[0],
            y = at[1],
            c = css(color),
            a = angle.to_degrees(),
            t = esc(text)
        );
    }
}
