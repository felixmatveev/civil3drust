//! Camera and drawing, independent of the GUI toolkit.
//!
//! [`draw`] turns a [`Scene`] into calls on a [`Canvas`]; the window
//! implements it with egui's painter and `--svg` with an SVG writer, so both
//! show exactly the same picture.

use crate::scene::{Colored, Decoded, Prim, Scene, P3};

/// Screen-space point, pixels, y down.
pub type S2 = [f32; 2];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

pub trait Canvas {
    fn polyline(&mut self, pts: &[S2], closed: bool, color: Rgba, width: f32);
    fn marker(&mut self, at: S2, color: Rgba, size: f32);
    /// Text with its baseline starting at `at`, rotated `angle` radians
    /// clockwise on screen, `size` pixels high.
    fn text(&mut self, at: S2, angle: f32, size: f32, text: &str, color: Rgba);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Plan,
    Orbit,
}

/// Plan view (2D) and orbit view (3D) share one camera: a target point,
/// a zoom expressed as pixels per drawing unit at the target, and for 3D a
/// yaw and pitch.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub mode: Mode,
    pub target: P3,
    pub px_per_unit: f64,
    /// Radians; 0 looks along +Y (north), increasing turns the view left.
    pub yaw: f64,
    /// Radians above the horizon, 0..π/2.
    pub pitch: f64,
    pub perspective: bool,
    pub viewport: [f32; 4], // x, y, w, h in pixels
}

impl Camera {
    pub fn new() -> Self {
        Camera {
            mode: Mode::Plan,
            target: [0.0; 3],
            px_per_unit: 1.0,
            yaw: -0.6,
            pitch: 0.6,
            perspective: true,
            viewport: [0.0, 0.0, 800.0, 600.0],
        }
    }

    fn center(&self) -> [f64; 2] {
        let v = self.viewport;
        [(v[0] + v[2] / 2.0) as f64, (v[1] + v[3] / 2.0) as f64]
    }

    /// Camera basis for the orbit view: right, up, forward (into screen).
    fn basis(&self) -> (P3, P3, P3) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        // Looking from the eye towards the target.
        let fwd = [-sy * cp, cy * cp, -sp];
        let right = [cy, sy, 0.0];
        let up = cross(right, fwd);
        (right, up, fwd)
    }

    /// Distance from eye to target in drawing units (orbit, perspective).
    fn eye_distance(&self) -> f64 {
        // A 50° vertical field of view keeps `px_per_unit` meaning the
        // same at the target in both projections.
        let f = self.focal_px();
        f / self.px_per_unit
    }

    fn focal_px(&self) -> f64 {
        self.viewport[3] as f64 / 2.0 / (25f64.to_radians()).tan()
    }

    /// Project a world point. `None` if behind the camera.
    pub fn project(&self, p: P3) -> Option<S2> {
        let [cx, cy] = self.center();
        let d = [
            p[0] - self.target[0],
            p[1] - self.target[1],
            p[2] - self.target[2],
        ];
        match self.mode {
            Mode::Plan => Some([
                (cx + d[0] * self.px_per_unit) as f32,
                (cy - d[1] * self.px_per_unit) as f32,
            ]),
            Mode::Orbit => {
                let (r, u, f) = self.basis();
                let x = dot(d, r);
                let y = dot(d, u);
                let z = dot(d, f);
                if self.perspective {
                    let depth = self.eye_distance() + z;
                    if depth <= self.eye_distance() * 0.02 {
                        return None;
                    }
                    let k = self.focal_px() / depth;
                    Some([(cx + x * k) as f32, (cy - y * k) as f32])
                } else {
                    Some([
                        (cx + x * self.px_per_unit) as f32,
                        (cy - y * self.px_per_unit) as f32,
                    ])
                }
            }
        }
    }

    /// Screen direction of a world vector at a world point, as an angle
    /// (clockwise, radians) and a length factor (pixels per world unit).
    fn screen_dir(&self, at: P3, dir: P3) -> Option<(f32, f32)> {
        let a = self.project(at)?;
        let b = self.project([at[0] + dir[0], at[1] + dir[1], at[2] + dir[2]])?;
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        Some((dy.atan2(dx), (dx * dx + dy * dy).sqrt()))
    }

    /// Plan-view world point under a screen position (z = 0 plane in 3D
    /// is not solved; the status bar shows coordinates only in plan).
    pub fn unproject_plan(&self, s: S2) -> [f64; 2] {
        let [cx, cy] = self.center();
        [
            self.target[0] + (s[0] as f64 - cx) / self.px_per_unit,
            self.target[1] - (s[1] as f64 - cy) / self.px_per_unit,
        ]
    }

    /// Pan by a screen delta.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let s = 1.0 / self.px_per_unit;
        match self.mode {
            Mode::Plan => {
                self.target[0] -= dx as f64 * s;
                self.target[1] += dy as f64 * s;
            }
            Mode::Orbit => {
                let (r, u, _) = self.basis();
                for i in 0..3 {
                    self.target[i] += (-r[i] * dx as f64 + u[i] * dy as f64) * s;
                }
            }
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx as f64 * 0.008;
        self.pitch = (self.pitch + dy as f64 * 0.008).clamp(0.02, std::f64::consts::FRAC_PI_2);
    }

    /// Zoom by `factor` keeping the plan point under `at` fixed.
    pub fn zoom_at(&mut self, factor: f64, at: Option<S2>) {
        let before = at.map(|s| self.unproject_plan(s));
        self.px_per_unit = (self.px_per_unit * factor).clamp(1e-6, 1e9);
        if let (Mode::Plan, Some(b), Some(s)) = (self.mode, before, at) {
            let after = self.unproject_plan(s);
            self.target[0] += b[0] - after[0];
            self.target[1] += b[1] - after[1];
        }
    }

    /// Frame the bounding box `[min, max]`.
    pub fn fit(&mut self, b: Option<(P3, P3)>) {
        let Some((lo, hi)) = b else {
            self.target = [0.0; 3];
            self.px_per_unit = 1.0;
            return;
        };
        self.target = [
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ];
        let w = (hi[0] - lo[0]).max(1e-6);
        let h = (hi[1] - lo[1]).max(1e-6);
        let size = match self.mode {
            Mode::Plan => (w / self.viewport[2] as f64).max(h / self.viewport[3] as f64),
            Mode::Orbit => {
                let d = (w * w + h * h + (hi[2] - lo[2]).powi(2)).sqrt();
                d / (self.viewport[2].min(self.viewport[3]) as f64) * 0.7
            }
        };
        self.px_per_unit = 0.85 / size.max(1e-9);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P3, b: P3) -> P3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// What to draw.
#[derive(Debug, Clone)]
pub struct Layers {
    pub decoded: bool,
    pub proxy: bool,
    /// Draw Civil 3D's "not drawn yet" placeholder images too.
    pub placeholders: bool,
    pub context: bool,
    pub text: bool,
    pub grid: bool,
    /// Lift the proxy layer by this many units in 3D so it sits above
    /// the decoded geometry instead of on top of it.
    pub proxy_lift: f64,
    /// Classes switched off in the object list.
    pub hidden_classes: std::collections::BTreeSet<String>,
    pub selected: Option<u64>,
    pub dark: bool,
}

impl Default for Layers {
    fn default() -> Self {
        Layers {
            decoded: true,
            proxy: true,
            placeholders: false,
            context: true,
            text: true,
            grid: true,
            proxy_lift: 0.0,
            hidden_classes: Default::default(),
            selected: None,
            dark: true,
        }
    }
}

pub struct Palette {
    pub bg: Rgba,
    pub grid: Rgba,
    pub grid_axis: Rgba,
    pub decoded: Rgba,
    pub decoded_text: Rgba,
    pub proxy: Rgba,
    pub context: Rgba,
    pub selected: Rgba,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Palette {
                bg: Rgba(24, 26, 31, 255),
                grid: Rgba(255, 255, 255, 18),
                grid_axis: Rgba(255, 255, 255, 50),
                decoded: Rgba(255, 170, 40, 255),
                decoded_text: Rgba(255, 200, 120, 255),
                proxy: Rgba(80, 220, 230, 230),
                context: Rgba(150, 155, 165, 160),
                selected: Rgba(255, 255, 255, 255),
            }
        } else {
            Palette {
                bg: Rgba(250, 250, 248, 255),
                grid: Rgba(0, 0, 0, 16),
                grid_axis: Rgba(0, 0, 0, 50),
                decoded: Rgba(214, 110, 0, 255),
                decoded_text: Rgba(160, 80, 0, 255),
                proxy: Rgba(0, 140, 160, 230),
                context: Rgba(120, 120, 120, 170),
                selected: Rgba(20, 20, 20, 255),
            }
        }
    }
}

/// AutoCAD color index → RGB for the standard colors; other indexes fall
/// back to the layer color.
fn aci_rgb(aci: Option<i32>, fallback: Rgba, dark: bool) -> Rgba {
    let a = fallback.3;
    match aci {
        Some(1) => Rgba(235, 64, 52, a),
        Some(2) => Rgba(240, 220, 40, a),
        Some(3) => Rgba(60, 200, 80, a),
        Some(4) => Rgba(60, 210, 230, a),
        Some(5) => Rgba(70, 110, 240, a),
        Some(6) => Rgba(220, 70, 220, a),
        Some(7) if dark => Rgba(235, 235, 235, a),
        Some(7) => Rgba(30, 30, 30, a),
        Some(8) => Rgba(128, 128, 128, a),
        Some(9) => Rgba(192, 192, 192, a),
        _ => fallback,
    }
}

/// Bounding box of everything that would be drawn with these layers.
pub fn bounds(scene: &Scene, layers: &Layers) -> Option<(P3, P3)> {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut add = |p: &P3| {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    };
    let mut add_prim = |p: &Prim| match p {
        Prim::Line { pts, .. } => pts.iter().for_each(&mut add),
        Prim::Marker { at } | Prim::Text { at, .. } => add(at),
    };
    visit(scene, layers, |item| add_prim(item.prim));
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// A primitive about to be drawn and how.
struct Item<'a> {
    prim: &'a Prim,
    color: Rgba,
    width: f32,
    lift: f64,
    owner: Option<u64>,
}

/// Every visible primitive, in drawing order (context, proxy, decoded,
/// selection on top).
fn visit<'a>(scene: &'a Scene, layers: &Layers, mut f: impl FnMut(Item<'a>)) {
    let pal = Palette::new(layers.dark);
    if layers.context {
        for c in &scene.context {
            f(Item {
                prim: &c.prim,
                color: aci_rgb(c.aci, pal.context, layers.dark).with_alpha(pal.context.3),
                width: 1.0,
                lift: 0.0,
                owner: None,
            });
        }
    }
    let visible = |o: &&crate::scene::Object| !layers.hidden_classes.contains(&o.class);
    let mut selected: Vec<Item<'a>> = Vec::new();
    if layers.proxy {
        for o in scene.objects.iter().filter(visible) {
            let Some(g) = &o.proxy else { continue };
            if g.placeholder && !layers.placeholders {
                continue;
            }
            let sel = layers.selected == Some(o.handle);
            for Colored { prim, aci } in &g.prims {
                if matches!(prim, Prim::Text { .. }) && !layers.text {
                    continue;
                }
                let item = Item {
                    prim,
                    color: if sel {
                        pal.selected
                    } else {
                        aci_rgb(*aci, pal.proxy, layers.dark)
                    },
                    width: if sel { 2.0 } else { 1.2 },
                    lift: layers.proxy_lift,
                    owner: Some(o.handle),
                };
                if sel {
                    selected.push(item)
                } else {
                    f(item)
                }
            }
        }
    }
    if layers.decoded {
        for o in scene.objects.iter().filter(visible) {
            let Decoded::Alignment(a) = &o.decoded else {
                continue;
            };
            let sel = layers.selected == Some(o.handle);
            for (i, prim) in a.prims.iter().enumerate() {
                let is_text = matches!(prim, Prim::Text { .. });
                if is_text && !layers.text {
                    continue;
                }
                let item = Item {
                    prim,
                    color: match (sel, is_text) {
                        (true, _) => pal.selected,
                        (false, true) => pal.decoded_text,
                        (false, false) => pal.decoded,
                    },
                    width: match (sel, i) {
                        (true, 0) => 3.5,
                        (false, 0) => 2.5,
                        _ => 1.3,
                    },
                    lift: 0.0,
                    owner: Some(o.handle),
                };
                if sel {
                    selected.push(item)
                } else {
                    f(item)
                }
            }
        }
    }
    selected.into_iter().for_each(f);
}

impl Rgba {
    pub fn with_alpha(self, a: u8) -> Rgba {
        Rgba(self.0, self.1, self.2, a)
    }
}

/// Draw the scene. Returns, for picking, every drawn polyline as screen
/// points with its owner handle.
pub fn draw(
    scene: &Scene,
    layers: &Layers,
    cam: &Camera,
    canvas: &mut dyn Canvas,
) -> Vec<(u64, Vec<S2>)> {
    let pal = Palette::new(layers.dark);
    if layers.grid {
        draw_grid(scene, layers, cam, canvas, &pal);
    }
    let mut pick = Vec::new();
    visit(scene, layers, |it| {
        let lift = |p: &P3| [p[0], p[1], p[2] + it.lift];
        match it.prim {
            Prim::Line { pts, closed } => {
                // Split where points fall behind the camera.
                let mut run: Vec<S2> = Vec::new();
                let mut all = true;
                for p in pts {
                    match cam.project(lift(p)) {
                        Some(s) => run.push(s),
                        None => {
                            all = false;
                            if run.len() >= 2 {
                                canvas.polyline(&run, false, it.color, it.width);
                            }
                            run.clear();
                        }
                    }
                }
                if run.len() >= 2 {
                    canvas.polyline(&run, *closed && all, it.color, it.width);
                    if let Some(h) = it.owner {
                        let mut r = run.clone();
                        if *closed && all {
                            r.push(run[0]);
                        }
                        pick.push((h, r));
                    }
                }
            }
            Prim::Marker { at } => {
                if let Some(s) = cam.project(lift(at)) {
                    canvas.marker(s, it.color, 3.5);
                    if let Some(h) = it.owner {
                        pick.push((h, vec![s]));
                    }
                }
            }
            Prim::Text {
                at,
                dir,
                height,
                text,
            } => {
                let p = lift(at);
                let Some(s) = cam.project(p) else { return };
                let Some((angle, k)) = cam.screen_dir(p, *dir) else {
                    return;
                };
                let size = (*height as f32) * k;
                if (4.0..400.0).contains(&size) {
                    let (s, angle) = upright(s, angle, size, text);
                    canvas.text(s, angle, size, text, it.color);
                }
            }
        }
    });
    pick
}

/// Turn text that would read upside down by half a turn, keeping it over
/// the same footprint (width estimated from the character count).
fn upright(at: S2, angle: f32, size: f32, text: &str) -> (S2, f32) {
    use std::f32::consts::{FRAC_PI_2, PI};
    if angle > -FRAC_PI_2 - 1e-3 && angle <= FRAC_PI_2 + 1e-3 {
        return (at, angle);
    }
    let w = 0.55 * size * text.chars().count() as f32;
    let (s, c) = angle.sin_cos();
    // Baseline end, then up by one text height (screen y points down).
    let at = [at[0] + c * w + s * size, at[1] + s * w - c * size];
    (at, angle - PI)
}

/// Ground grid at z = 0 with a step that keeps lines 40–400 px apart.
fn draw_grid(scene: &Scene, layers: &Layers, cam: &Camera, canvas: &mut dyn Canvas, pal: &Palette) {
    let step = {
        let target_px = 80.0;
        let raw = target_px / cam.px_per_unit;
        let p = 10f64.powf(raw.log10().floor());
        [1.0, 2.0, 5.0, 10.0]
            .iter()
            .map(|m| m * p)
            .find(|s| *s >= raw)
            .unwrap_or(10.0 * p)
    };
    let half = match cam.mode {
        Mode::Plan => (cam.viewport[2].max(cam.viewport[3]) as f64 / cam.px_per_unit) * 0.75,
        Mode::Orbit => {
            let b = bounds(scene, layers);
            let span = b
                .map(|(lo, hi)| (hi[0] - lo[0]).max(hi[1] - lo[1]))
                .unwrap_or(100.0);
            (span * 0.8).max(step * 5.0)
        }
    };
    let n = ((half / step).ceil() as i64).min(400);
    let c = [
        (cam.target[0] / step).round() * step,
        (cam.target[1] / step).round() * step,
    ];
    for i in -n..=n {
        let x = c[0] + i as f64 * step;
        let y = c[1] + i as f64 * step;
        for (a, b, on_axis) in [
            (
                [x, c[1] - half, 0.0],
                [x, c[1] + half, 0.0],
                x.abs() < step * 1e-6,
            ),
            (
                [c[0] - half, y, 0.0],
                [c[0] + half, y, 0.0],
                y.abs() < step * 1e-6,
            ),
        ] {
            let pts: Vec<S2> = subdivide(a, b, 16)
                .into_iter()
                .filter_map(|p| cam.project(p))
                .collect();
            if pts.len() >= 2 {
                let col = if on_axis { pal.grid_axis } else { pal.grid };
                canvas.polyline(&pts, false, col, 1.0);
            }
        }
    }
}

fn subdivide(a: P3, b: P3, n: usize) -> Vec<P3> {
    (0..=n)
        .map(|i| {
            let t = i as f64 / n as f64;
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        })
        .collect()
}

/// Handle of the drawn object nearest `at`, within `tol` pixels.
pub fn pick(lines: &[(u64, Vec<S2>)], at: S2, tol: f32) -> Option<u64> {
    let mut best: Option<(f32, u64)> = None;
    for (h, pts) in lines {
        let d = if pts.len() == 1 {
            dist(pts[0], at)
        } else {
            pts.windows(2)
                .map(|w| seg_dist(w[0], w[1], at))
                .fold(f32::INFINITY, f32::min)
        };
        if d <= tol && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, *h));
        }
    }
    best.map(|(_, h)| h)
}

fn dist(a: S2, b: S2) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn seg_dist(a: S2, b: S2, p: S2) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    if l2 <= 0.0 {
        return dist(a, p);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
    dist([a[0] + dx * t, a[1] + dy * t], p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_projection_round_trip() {
        let mut c = Camera::new();
        c.target = [100.0, 50.0, 0.0];
        c.px_per_unit = 2.0;
        let s = c.project([110.0, 40.0, 0.0]).unwrap();
        assert_eq!(s, [420.0, 320.0]);
        let w = c.unproject_plan(s);
        assert!((w[0] - 110.0).abs() < 1e-9 && (w[1] - 40.0).abs() < 1e-9);
    }

    #[test]
    fn orbit_target_projects_to_center() {
        let mut c = Camera::new();
        c.mode = Mode::Orbit;
        c.target = [5.0, 5.0, 1.0];
        let s = c.project(c.target).unwrap();
        assert!((s[0] - 400.0).abs() < 1e-3 && (s[1] - 300.0).abs() < 1e-3);
        // Up is up: a point above the target lands above the centre.
        let up = c.project([5.0, 5.0, 2.0]).unwrap();
        assert!(up[1] < s[1]);
    }

    #[test]
    fn pick_nearest() {
        let lines = vec![
            (1, vec![[0.0, 0.0], [100.0, 0.0]]),
            (2, vec![[0.0, 10.0], [100.0, 10.0]]),
        ];
        assert_eq!(pick(&lines, [50.0, 8.0], 5.0), Some(2));
        assert_eq!(pick(&lines, [50.0, 50.0], 5.0), None);
    }
}
