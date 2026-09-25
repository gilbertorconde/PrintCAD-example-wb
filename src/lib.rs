//! Spacers: an example printCAD workbench package.
//!
//! A spacer is a feature of its own kind: a round or hex prism with a bore
//! through it. The package shows the parts of a workbench most packages
//! need: tools that make features, a task panel whose numbers take
//! formulas, a rebuild plan the kernel runs, drawing in the view, a command
//! for scripts and agents, a tree menu entry and a settings page.

use printcad_bench_sdk::api::kernel_api::{
    BooleanOp, ExtrudeTermination, Profile, ProfilePlane, ProfileSegment, ProfileWire, SweepKind,
};
use printcad_bench_sdk::api::*;
use printcad_bench_sdk::{Bench, Value, bench, host, json, serde_json};
use serde::{Deserialize, Serialize};

const KIND: &str = "example.spacers.spacer";
const ROUND: &str = "example.spacers.round";
const HEX: &str = "example.spacers.hex";
const MAKE: &str = "example.spacers.make";
const EDIT: &str = "example.spacers.edit";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Round,
    Hex,
}

/// A spacer's data. Lengths in millimetres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spacer {
    pub shape: Shape,
    /// The diameter, or the width across flats of a hex.
    pub outer: f64,
    pub bore: f64,
    pub height: f64,
}

impl Spacer {
    fn from(data: &Value) -> Option<Spacer> {
        serde_json::from_value(data.clone()).ok()
    }

    fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// What is wrong with it, if anything.
    pub fn problem(&self) -> Option<&'static str> {
        if self.outer <= 0.0 || self.height <= 0.0 {
            return Some("The outside and the height must be more than 0.");
        }
        if self.bore < 0.0 {
            return Some("The bore cannot be less than 0.");
        }
        // Leave at least 0.4 mm of wall, a nozzle's width.
        if self.bore > 0.0 && self.outer - self.bore < 0.8 {
            return Some("The wall around the bore is thinner than 0.4 mm.");
        }
        None
    }

    /// The outside outline, on the XY plane about the origin.
    pub fn outline(&self) -> Vec<ProfileSegment> {
        match self.shape {
            Shape::Round => vec![ProfileSegment::Circle {
                center: [0.0, 0.0],
                radius: self.outer / 2.0,
            }],
            Shape::Hex => {
                // Across flats `outer`: the corners sit on a circle of
                // radius outer / √3, flats square to X.
                let r = self.outer / 3f64.sqrt();
                let corner = |i: usize| {
                    let a = std::f64::consts::FRAC_PI_3 * i as f64;
                    [r * a.cos(), r * a.sin()]
                };
                (0..6)
                    .map(|i| ProfileSegment::Line {
                        start: corner(i),
                        end: corner((i + 1) % 6),
                    })
                    .collect()
            }
        }
    }

    /// The kernel op that builds it.
    pub fn op(&self, op: BooleanOp) -> SolidOp {
        let mut wires = vec![ProfileWire {
            segments: self.outline(),
        }];
        if self.bore > 0.0 {
            wires.push(ProfileWire {
                segments: vec![ProfileSegment::Circle {
                    center: [0.0, 0.0],
                    radius: self.bore / 2.0,
                }],
            });
        }
        SolidOp::Sweep {
            profile: Profile {
                plane: ProfilePlane {
                    origin: [0.0; 3],
                    x_axis: [1.0, 0.0, 0.0],
                    y_axis: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                },
                wires,
            },
            kind: SweepKind::Extrude {
                termination: ExtrudeTermination::Blind {
                    distance: self.height,
                },
                second_side: None,
                symmetric: false,
                reversed: false,
                taper_deg: 0.0,
                direction: None,
            },
            op,
        }
    }

    fn label(&self) -> String {
        let shape = match self.shape {
            Shape::Round => "Ø",
            Shape::Hex => "⬡",
        };
        format!("{shape}{} × {}", trim(self.outer), trim(self.height))
    }
}

/// A number without trailing zeros.
fn trim(v: f64) -> String {
    let text = format!("{v:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// What a new spacer starts from.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Defaults {
    outer: f64,
    bore: f64,
    height: f64,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            outer: 8.0,
            bore: 3.4,
            height: 10.0,
        }
    }
}

#[derive(Default)]
struct Spacers {
    /// The spacer whose task is open, with its data when it opened.
    editing: Option<(String, Spacer)>,
    /// The tool just made it: Cancel removes it.
    fresh: bool,
    defaults: Defaults,
}

impl Spacers {
    fn new_spacer(&self, shape: Shape) -> Spacer {
        Spacer {
            shape,
            outer: self.defaults.outer,
            bore: self.defaults.bore,
            height: self.defaults.height,
        }
    }

    /// A body with the spacer on it: `(body, feature)`.
    fn make(&self, spacer: &Spacer) -> Result<(String, String), String> {
        let body = host::create_body(Some("Spacer"))?;
        let feature = host::add_feature(KIND, &spacer.label(), Some(&body), spacer.to_value())?;
        Ok((body, feature))
    }

    fn open(&mut self, id: &str, fresh: bool) {
        if let Some(node) = host::feature(id)
            && node.kind == KIND
            && let Some(spacer) = Spacer::from(&node.data)
        {
            self.editing = Some((id.to_string(), spacer));
            self.fresh = fresh;
        }
    }

    /// The spacer under edit as it is now.
    fn edited(&self) -> Option<(String, Spacer)> {
        let (id, _) = self.editing.as_ref()?;
        let node = host::feature(id)?;
        Some((id.clone(), Spacer::from(&node.data)?))
    }
}

impl Bench for Spacers {
    fn describe(&self) -> Registration {
        let tool = |id: &str, label: &str, icon: &str, key: &str| Tool {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            behavior: ToolBehavior::Action,
            shortcuts: vec![key.into()],
            ..Default::default()
        };
        let param = |name: &str, kind: ParamKind, required: bool, doc: &str| Param {
            name: name.into(),
            kind,
            required,
            doc: doc.into(),
        };
        Registration {
            label: "Spacers".into(),
            description: "Round and hex spacers and standoffs".into(),
            icon: "spacer".into(),
            tools: vec![
                tool(ROUND, "Round spacer", "spacer", "R"),
                tool(HEX, "Hex spacer", "spacer-hex", "H"),
            ],
            commands: vec![Command {
                id: MAKE.into(),
                summary: "Make a spacer on a body of its own".into(),
                params: vec![
                    param(
                        "shape",
                        ParamKind::String,
                        false,
                        "`round` (the default) or `hex`",
                    ),
                    param(
                        "outer",
                        ParamKind::Number,
                        true,
                        "diameter or across flats, mm",
                    ),
                    param(
                        "bore",
                        ParamKind::Number,
                        false,
                        "hole diameter, mm; 0 for none",
                    ),
                    param("height", ParamKind::Number, true, "mm"),
                ],
                returns: "{body, feature}".into(),
                read_only: false,
            }],
            length_keys: vec!["outer".into(), "bore".into(), "height".into()],
            ..Default::default()
        }
    }

    fn feature_info(&self, node: &Node) -> FeatureInfo {
        let spacer = Spacer::from(&node.data);
        FeatureInfo {
            icon: match spacer.as_ref().map(|s| s.shape) {
                Some(Shape::Hex) => "spacer-hex".into(),
                _ => "spacer".into(),
            },
            kind_label: match spacer {
                Some(s) => format!("Spacer {}", s.label()),
                None => "Spacer".into(),
            },
            family_label: "Spacer".into(),
            builds_solid: true,
        }
    }

    fn parameters(&self, _node: &Node) -> Vec<Parameter> {
        let length = |key: &str, label: &str| Parameter {
            key: format!("/{key}"),
            name: Some(key.into()),
            label: label.into(),
            dim: Dim::Length,
            pointer: format!("/{key}"),
            scale: 1.0,
            integer: false,
        };
        vec![
            length("outer", "Outside"),
            length("bore", "Bore"),
            length("height", "Height"),
        ]
    }

    fn rebuild(&mut self, request: RebuildRequest) -> Vec<Rebuild> {
        request
            .bodies
            .into_iter()
            .map(|history| {
                let mut ops = Vec::new();
                let mut op_features = Vec::new();
                for node in &history.features {
                    let fail = |message: &str| Rebuild {
                        body: history.body.clone(),
                        plan: Plan::Error {
                            feature: Some(node.id.clone()),
                            message: message.into(),
                        },
                    };
                    let Some(spacer) = Spacer::from(&node.data) else {
                        return fail("The spacer's data does not read.");
                    };
                    if let Some(problem) = spacer.problem() {
                        return fail(problem);
                    }
                    let op = if ops.is_empty() {
                        BooleanOp::NewSolid
                    } else {
                        BooleanOp::Fuse
                    };
                    ops.push(spacer.op(op));
                    op_features.push(node.id.clone());
                }
                Rebuild {
                    body: history.body,
                    plan: if ops.is_empty() {
                        Plan::Empty
                    } else {
                        Plan::Ops { ops, op_features }
                    },
                }
            })
            .collect()
    }

    fn run_command(&mut self, id: &str, args: Value) -> Result<Value, String> {
        if id != MAKE {
            return Err(format!("no command `{id}`"));
        }
        let number =
            |key: &str, default: f64| args.get(key).and_then(Value::as_f64).unwrap_or(default);
        let shape = match args.get("shape").and_then(Value::as_str) {
            None | Some("round") => Shape::Round,
            Some("hex") => Shape::Hex,
            Some(other) => return Err(format!("`{other}` is not a shape: `round` or `hex`")),
        };
        let spacer = Spacer {
            shape,
            outer: number("outer", self.defaults.outer),
            bore: number("bore", self.defaults.bore),
            height: number("height", self.defaults.height),
        };
        if let Some(problem) = spacer.problem() {
            return Err(problem.into());
        }
        let (body, feature) = self.make(&spacer)?;
        Ok(json!({"body": body, "feature": feature}))
    }

    fn input(&mut self, input: &Input) -> bool {
        match &input.event {
            Event::ToolActivated => {
                let shape = match input.tool.as_deref() {
                    Some(ROUND) => Shape::Round,
                    Some(HEX) => Shape::Hex,
                    _ => return false,
                };
                match self.make(&self.new_spacer(shape)) {
                    Ok((body, feature)) => {
                        host::request(Request::SelectBody { body });
                        host::request(Request::JournalLabel {
                            label: "New spacer".into(),
                        });
                        self.open(&feature, true);
                    }
                    Err(e) => host::error(&format!("No spacer: {e}")),
                }
                true
            }
            Event::Key { key, down: true } if key == "Escape" && self.editing.is_some() => {
                self.task_close(false);
                true
            }
            _ => false,
        }
    }

    fn frame(&mut self, _pointer: &Pointer) -> Frame {
        let mut frame = Frame::default();
        let Some((id, spacer)) = self.edited() else {
            self.editing = None;
            return frame;
        };
        // The spacer's axis and size, where its body sits.
        let placement = host::feature(&id)
            .and_then(|n| n.body)
            .and_then(|b| host::bodies().into_iter().find(|x| x.id == b))
            .map(|b| b.placement);
        let world = |z: f64| -> [f32; 3] {
            match placement {
                Some(m) => [
                    (m[2] * z + m[3]) as f32,
                    (m[6] * z + m[7]) as f32,
                    (m[10] * z + m[11]) as f32,
                ],
                None => [0.0, 0.0, z as f32],
            }
        };
        frame.lines.push(Polyline {
            points: vec![world(-2.0), world(spacer.height + 2.0)],
            color: [0.31, 0.64, 0.9],
            width: 1.5,
            dashed: true,
            closed: false,
        });
        frame.labels.push(Label {
            at: world(spacer.height + 3.0),
            text: spacer.label(),
            color: [0.9, 0.92, 0.94],
            size: 12.0,
            pill: true,
            mono: true,
        });
        frame.hud.tool = Some(ToolHint {
            icon: "spacer".into(),
            name: "Spacer".into(),
            prompt: "Set its size in the panel".into(),
            keys: vec![("Esc".into(), "cancel".into())],
        });
        frame.editing = Some(id.clone());
        frame.task = Some(Task {
            title: "Spacer".into(),
            icon: "spacer".into(),
            confirmable: true,
        });
        let number = |key: &str, label: &str, value: f64| Widget::Number {
            id: key.into(),
            label: label.into(),
            value,
            dim: Dim::Length,
            bind: Some(Bind {
                feature: id.clone(),
                key: format!("/{key}"),
            }),
            min: Some(0.0),
            max: None,
            decimals: 2,
            error: None,
        };
        frame.panel = vec![
            Widget::Choice {
                id: "shape".into(),
                label: "Shape".into(),
                options: vec!["Round".into(), "Hex".into()],
                selected: match spacer.shape {
                    Shape::Round => 0,
                    Shape::Hex => 1,
                },
            },
            number(
                "outer",
                match spacer.shape {
                    Shape::Round => "Diameter",
                    Shape::Hex => "Across flats",
                },
                spacer.outer,
            ),
            number("bore", "Bore", spacer.bore),
            number("height", "Height", spacer.height),
        ];
        if let Some(problem) = spacer.problem() {
            frame.panel.push(Widget::Note {
                kind: NoteKind::Error,
                title: None,
                text: problem.into(),
            });
        }
        frame
    }

    fn panel_event(&mut self, slot: PanelSlot, event: PanelEvent) {
        if slot == PanelSlot::Settings {
            if let PanelEvent::Number { id, value } = event {
                match id.as_str() {
                    "outer" if value > 0.0 => self.defaults.outer = value,
                    "bore" if value >= 0.0 => self.defaults.bore = value,
                    "height" if value > 0.0 => self.defaults.height = value,
                    _ => {}
                }
            }
            return;
        }
        let Some((id, mut spacer)) = self.edited() else {
            return;
        };
        match event {
            PanelEvent::Choice { index, .. } => {
                spacer.shape = if index == 1 { Shape::Hex } else { Shape::Round }
            }
            PanelEvent::Number { id: field, value } => match field.as_str() {
                "outer" => spacer.outer = value,
                "bore" => spacer.bore = value,
                "height" => spacer.height = value,
                _ => return,
            },
            _ => return,
        }
        if let Err(e) = host::set_feature_data(&id, spacer.to_value()) {
            host::error(&e);
        }
    }

    fn task_close(&mut self, accept: bool) -> Option<String> {
        let (id, opened) = self.editing.take()?;
        if accept {
            return Some("Edit spacer".into());
        }
        let undone = if self.fresh {
            host::remove_feature(&id)
        } else {
            host::set_feature_data(&id, opened.to_value())
        };
        if let Err(e) = undone {
            host::error(&e);
        }
        None
    }

    fn menu_items(&mut self, scope: &MenuScope) -> Vec<MenuItem> {
        match scope {
            MenuScope::TreeFeature(id) if host::feature(id).is_some_and(|n| n.kind == KIND) => {
                vec![MenuItem {
                    id: EDIT.into(),
                    label: "Edit spacer".into(),
                    icon: Some("spacer".into()),
                    hint: None,
                    enabled: true,
                    separator_before: false,
                }]
            }
            _ => Vec::new(),
        }
    }

    fn menu_command(&mut self, id: &str, scope: &MenuScope) -> bool {
        match (id, scope) {
            (EDIT, MenuScope::TreeFeature(feature)) => {
                self.open(feature, false);
                true
            }
            _ => false,
        }
    }

    fn settings_panel(&mut self) -> Vec<Widget> {
        let number = |key: &str, label: &str, value: f64| Widget::Number {
            id: key.into(),
            label: label.into(),
            value,
            dim: Dim::Length,
            bind: None,
            min: Some(0.0),
            max: None,
            decimals: 2,
            error: None,
        };
        vec![
            Widget::Heading {
                text: "New spacers".into(),
            },
            number("outer", "Outside", self.defaults.outer),
            number("bore", "Bore", self.defaults.bore),
            number("height", "Height", self.defaults.height),
        ]
    }

    fn settings(&self) -> Option<Value> {
        serde_json::to_value(&self.defaults).ok()
    }

    fn apply_settings(&mut self, settings: Value) {
        if let Ok(defaults) = serde_json::from_value(settings) {
            self.defaults = defaults;
        }
    }
}

bench!(Spacers);

#[cfg(test)]
mod tests {
    use super::*;

    fn spacer(shape: Shape) -> Spacer {
        Spacer {
            shape,
            outer: 8.0,
            bore: 3.4,
            height: 10.0,
        }
    }

    #[test]
    fn a_hex_is_as_wide_across_its_flats_as_asked() {
        let segments = spacer(Shape::Hex).outline();
        assert_eq!(segments.len(), 6);
        let mut max_y: f64 = 0.0;
        for s in &segments {
            let ProfileSegment::Line { start, end } = s else {
                panic!("a hex is lines");
            };
            max_y = max_y.max(start[1].abs()).max(end[1].abs());
        }
        assert!(
            (2.0 * max_y - 8.0).abs() < 1e-9,
            "across flats {}",
            2.0 * max_y
        );
    }

    #[test]
    fn a_spacer_needs_a_wall_a_nozzle_can_print() {
        assert_eq!(spacer(Shape::Round).problem(), None);
        let thin = Spacer {
            bore: 7.5,
            ..spacer(Shape::Round)
        };
        assert!(thin.problem().is_some());
        let flat = Spacer {
            height: 0.0,
            ..spacer(Shape::Hex)
        };
        assert!(flat.problem().is_some());
    }

    #[test]
    fn its_data_reads_back_as_written() {
        let s = spacer(Shape::Hex);
        assert_eq!(Spacer::from(&s.to_value()), Some(s));
        assert_eq!(spacer(Shape::Round).label(), "Ø8 × 10");
    }
}
