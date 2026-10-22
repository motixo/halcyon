//! First-run setup screens: the illustrated backdrop, one frosted card per
//! step and the drawn dock and panel choices. Brand colors on purpose; the
//! wizard looks the same whatever theme the desktop has yet.
use super::*;
use crate::desktop::profiles::Preset;
use crate::settings::wizard::{Message as Step, Shape, Step as Screen, Wizard};
use cosmic::iced::widget::stack;
use cosmic::widget::image::Handle;

const INK: Color = Color::from_rgb(0.95, 0.95, 1.);
const SOFT: Color = Color::from_rgb(0.72, 0.72, 0.84);
const LAVENDER: Color = Color::from_rgb(0.69, 0.63, 1.);
const PRIMARY: Color = Color::from_rgb(0.60, 0.53, 0.98);
const CARD: Color = Color::from_rgba(0.06, 0.07, 0.16, 0.84);
const TILE: Color = Color::from_rgba(1., 1., 1., 0.05);
const EDGE: Color = Color::from_rgba(1., 1., 1., 0.12);
/// The step card's widest; the window's sides show the backdrop.
const CARD_WIDTH: f32 = 1100.;
const DANGER: Color = Color::from_rgb(0.96, 0.58, 0.54);

fn go(message: Step) -> Message {
    Message::Wizard(message)
}

/// A wizard background, decoded once.
fn backdrop(welcome: bool) -> Option<Handle> {
    static DECODED: std::sync::OnceLock<[Option<Handle>; 2]> = std::sync::OnceLock::new();
    let decode = |bytes: &[u8]| {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).ok()?;
        let rgba = image.to_rgba8();
        Some(Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()))
    };
    DECODED.get_or_init(|| {
        [
            decode(include_bytes!("../../../assets/wizard/welcome.jpg")),
            decode(include_bytes!("../../../assets/wizard/backdrop.jpg")),
        ]
    })[usize::from(!welcome)]
    .clone()
}

/// The chosen look's wallpaper (or the wizard backdrop) cropped to
/// `aspect`, since a picture drawn larger than its box isn't clipped.
fn band(look: Option<&str>, aspect: f32) -> Option<Handle> {
    static CROPPED: std::sync::Mutex<Vec<((Option<String>, u32), Option<Handle>)>> =
        std::sync::Mutex::new(Vec::new());
    let key = (look.map(str::to_string), (aspect * 50.).round() as u32);
    let mut cropped = CROPPED.lock().ok()?;
    if let Some((_, handle)) = cropped.iter().find(|(known, _)| *known == key) {
        return handle.clone();
    }
    let source = match look.and_then(|id| super::profiles::look_rgba(id, true)) {
        Some(image) => Some(image),
        None => image::load_from_memory_with_format(
            include_bytes!("../../../assets/wizard/backdrop.jpg"),
            image::ImageFormat::Jpeg,
        )
        .ok()
        .map(|image| std::sync::Arc::new(image.to_rgba8())),
    };
    let handle = source.map(|image| {
        let (w, h) = (image.width(), image.height());
        let (cw, ch) = if w as f32 / h as f32 > aspect {
            (((h as f32) * aspect) as u32, h)
        } else {
            (w, ((w as f32) / aspect) as u32)
        };
        let part = image::imageops::crop_imm(&*image, (w - cw) / 2, (h - ch) / 2, cw, ch)
            .to_image();
        Handle::from_rgba(part.width(), part.height(), part.into_raw())
    });
    cropped.push((key, handle.clone()));
    handle
}

fn words<'a>(value: impl Into<String>, size: f32, color: Color) -> Element<'a, Message> {
    text::body(value.into())
        .size(size)
        .line_height(1.3)
        .class(cosmic::theme::Text::Color(color))
        .into()
}

fn heading<'a>(value: &'static str, size: f32) -> Element<'a, Message> {
    text::title1(value)
        .size(size)
        .line_height(1.15)
        .font(cosmic::font::bold())
        .class(cosmic::theme::Text::Color(INK))
        .into()
}

fn fill_box<'a>(
    content: Element<'a, Message>,
    color: Color,
    border: Color,
    radius: f32,
) -> cosmic::widget::Container<'a, Message, cosmic::Theme> {
    container(content).class(cosmic::theme::Container::custom(move |_| {
        cosmic::iced::widget::container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: radius.into(),
                width: if border.a > 0. { 1. } else { 0. },
                color: border,
            },
            ..Default::default()
        }
    }))
}

/// A plain drawn shape of `w` × `h`.
fn shape<'a>(w: f32, h: f32, radius: f32, color: Color) -> Element<'a, Message> {
    fill_box(
        cosmic::iced::widget::space().into(),
        color,
        Color::TRANSPARENT,
        radius,
    )
    .width(w)
    .height(h)
    .into()
}

fn button_style(
    background: Color,
    hover: Color,
    border: Color,
    radius: f32,
) -> cosmic::theme::Button {
    let style = move |focused: bool, hovered: bool| cosmic::widget::button::Style {
        background: Some(Background::Color(if hovered { hover } else { background })),
        border_color: border,
        border_width: if border.a > 0. { 1. } else { 0. },
        border_radius: radius.into(),
        outline_width: if focused { 2. } else { 0. },
        outline_color: LAVENDER,
        ..Default::default()
    };
    cosmic::theme::Button::Custom {
        active: Box::new(move |focused, _| style(focused, false)),
        hovered: Box::new(move |focused, _| style(focused, true)),
        pressed: Box::new(move |focused, _| style(focused, true)),
        disabled: Box::new(move |_| style(false, false)),
    }
}

fn primary<'a>(label: &'static str, id: &'static str, message: Message) -> Element<'a, Message> {
    button::custom(
        container(words(label, 14., Color::from_rgb(0.07, 0.06, 0.16)))
            .width(Length::Fill)
            .align_x(Alignment::Center),
    )
    .id(cosmic::widget::Id::new(id))
    .class(button_style(
        PRIMARY,
        Color::from_rgb(0.68, 0.62, 1.),
        Color::TRANSPARENT,
        18.,
    ))
    .padding([10, 18])
    .width(132.)
    .on_press(message)
    .into()
}

fn secondary<'a>(label: &'static str, id: &'static str, message: Message) -> Element<'a, Message> {
    button::custom(
        container(words(label, 14., INK))
            .width(Length::Fill)
            .align_x(Alignment::Center),
    )
    .id(cosmic::widget::Id::new(id))
    .class(button_style(
        Color::from_rgba(1., 1., 1., 0.07),
        Color::from_rgba(1., 1., 1., 0.13),
        EDGE,
        18.,
    ))
    .padding([10, 18])
    .width(96.)
    .on_press(message)
    .into()
}

fn quiet<'a>(label: &'static str, id: &'static str, message: Message) -> Element<'a, Message> {
    button::custom(words(label, 14., SOFT))
        .id(cosmic::widget::Id::new(id))
        .class(button_style(
            Color::TRANSPARENT,
            Color::from_rgba(1., 1., 1., 0.06),
            Color::TRANSPARENT,
            18.,
        ))
        .padding([10, 14])
        .on_press(message)
        .into()
}

fn dots<'a>(current: Screen) -> Element<'a, Message> {
    row::with_children(
        Screen::ALL
            .iter()
            .map(|step| {
                if *step == current {
                    shape(22., 8., 4., PRIMARY)
                } else {
                    shape(8., 8., 4., Color::from_rgba(1., 1., 1., 0.28))
                }
            })
            .collect::<Vec<_>>(),
    )
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// A choice drawn as a small picture with its name under it.
fn choice<'a>(
    picture: Element<'a, Message>,
    name: &'static str,
    id: String,
    selected: bool,
    message: Option<Message>,
) -> Element<'a, Message> {
    let enabled = message.is_some();
    button::custom(
        column::with_children(vec![
            container(picture)
                .width(Length::Fill)
                .height(34.)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
            words(name, 12., if enabled { INK } else { SOFT }),
        ])
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .id(cosmic::widget::Id::new(id))
    .class(button_style(
        if selected {
            Color::from_rgba(0.60, 0.53, 0.98, 0.16)
        } else {
            TILE
        },
        Color::from_rgba(1., 1., 1., 0.09),
        if selected { LAVENDER } else { EDGE },
        10.,
    ))
    .padding([8, 6])
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

fn corner(shape: Shape, size: f32) -> f32 {
    match shape {
        Shape::Pill => size / 2.,
        Shape::Rounded => (size / 3.).min(5.),
        Shape::Square => 0.,
    }
}

/// The panel drawn `width` wide: one bar, or three islands together.
fn panel_drawing<'a>(islands: bool, shape: Shape, width: f32, height: f32) -> Element<'a, Message> {
    let tone = Color::from_rgba(0.08, 0.08, 0.2, 0.88);
    let piece = |w: Length| -> Element<'a, Message> {
        fill_box(
            row::with_children(vec![
                shape_dot(height * 0.32),
                shape_dot(height * 0.32),
            ])
            .spacing(height * 0.2)
            .into(),
            tone,
            EDGE,
            corner(shape, height),
        )
        .width(w)
        .height(height)
        .padding([height * 0.34, height * 0.5])
        .into()
    };
    if islands {
        container(
            row::with_children(vec![
                piece(Length::Fixed(width * 0.16)),
                piece(Length::Fixed(width * 0.2)),
                piece(Length::Fixed(width * 0.14)),
            ])
            .spacing(height * 0.35),
        )
        .width(width)
        .align_x(Alignment::Center)
        .into()
    } else {
        container(piece(Length::Fill)).width(width).into()
    }
}

fn shape_dot<'a>(size: f32) -> Element<'a, Message> {
    shape(size, size, size / 2., Color::from_rgba(1., 1., 1., 0.75))
}

/// The dock: a short row of icons in its shape.
fn dock_drawing<'a>(shape: Shape, icon: f32) -> Element<'a, Message> {
    let icons = row::with_children((0..5).map(|_| shape_dot(icon)).collect::<Vec<_>>())
        .spacing(icon * 0.5);
    fill_box(
        icons.into(),
        Color::from_rgba(0.08, 0.08, 0.2, 0.88),
        EDGE,
        corner(shape, icon * 2.),
    )
    .padding([icon * 0.5, icon])
    .into()
}

impl Settings {
    /// The width inside the step card, beside its scroll bar.
    fn wizard_inner_width(&self) -> f32 {
        (self.window.width - 64.).min(CARD_WIDTH) - 56. - 12.
    }

    pub(super) fn wizard_view<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let welcome = wizard.step == Screen::Welcome;
        let mut layers: Vec<Element<'a, Message>> = Vec::new();
        layers.push(match backdrop(welcome) {
            Some(handle) => cosmic::widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Cover)
                .into(),
            None => shape(0., 0., 0., Color::from_rgb(0.05, 0.05, 0.14)),
        });
        layers.push(if welcome {
            self.wizard_welcome(wizard)
        } else {
            self.wizard_card(wizard)
        });
        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn wizard_welcome<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        // Setup resumed with changes made: continue, or undo them.
        let actions = if wizard.has_changes() {
            vec![
                primary("Continue", "wizard-start", go(Step::Start)),
                quiet("Cancel setup", "wizard-cancel", go(Step::Cancel)),
            ]
        } else {
            vec![
                primary("Start", "wizard-start", go(Step::Start)),
                quiet("Skip", "wizard-leave", go(Step::Leave)),
            ]
        };
        let intro = column::with_children(vec![
            heading("Halcyon", 64.),
            words(Screen::Welcome.hint(), 19., LAVENDER),
            cosmic::iced::widget::space().height(18.).into(),
            row::with_children(actions)
            .spacing(16)
            .align_y(Alignment::Center)
            .into(),
        ])
        .spacing(6);
        column::with_children(vec![
            container(intro)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([0, 64])
                .align_y(Alignment::Center)
                .into(),
            container(dots(Screen::Welcome))
                .width(Length::Fill)
                .padding([0, 0, 28, 0])
                .align_x(Alignment::Center)
                .into(),
        ])
        .into()
    }

    fn wizard_card<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let step = wizard.step;
        let body = match step {
            Screen::Look => self.wizard_looks(wizard),
            Screen::Bars => self.wizard_bars(wizard),
            Screen::Weather => self.wizard_weather(),
            Screen::Ready => self.wizard_ready(wizard),
            Screen::Welcome => cosmic::iced::widget::space().into(),
        };
        let mut parts = vec![
            heading(step.title(), 34.),
            words(step.hint(), 15., LAVENDER),
            cosmic::iced::widget::space().height(10.).into(),
            scrollable(container(body).padding([0, 12, 0, 0]))
                .id(cosmic::widget::Id::new("wizard-body"))
                .height(Length::Fill)
                .into(),
        ];
        if let Some(error) = &wizard.error {
            parts.push(words(error.clone(), 13., DANGER));
        }
        let (left, right): (Vec<Element<'a, Message>>, Element<'a, Message>) = match step {
            Screen::Ready => (
                vec![
                    secondary("Back", "wizard-back", go(Step::Back)),
                    quiet("Cancel", "wizard-cancel", go(Step::Cancel)),
                ],
                primary("Finish", "wizard-finish", go(Step::Finish)),
            ),
            _ => (
                vec![
                    secondary("Back", "wizard-back", go(Step::Back)),
                    quiet("Skip", "wizard-skip", go(Step::Skip)),
                ],
                // Cancel on every step undoes what setup changed so far.
                row::with_children(vec![
                    quiet("Cancel", "wizard-cancel", go(Step::Cancel)),
                    primary("Next", "wizard-next", go(Step::Next)),
                ])
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ),
        };
        parts.push(cosmic::iced::widget::space().height(6.).into());
        parts.push(
            row::with_children(vec![
                container(row::with_children(left).spacing(8).align_y(Alignment::Center))
                    .width(Length::FillPortion(2))
                    .into(),
                container(dots(step))
                    .width(Length::FillPortion(2))
                    .align_x(Alignment::Center)
                    .into(),
                container(right)
                    .width(Length::FillPortion(2))
                    .align_x(Alignment::End)
                    .into(),
            ])
            .align_y(Alignment::Center)
            .into(),
        );
        let card = fill_box(
            column::with_children(parts).spacing(4).into(),
            CARD,
            EDGE,
            18.,
        )
        .padding([26, 28])
        .max_width(CARD_WIDTH)
        .height(Length::Fill);
        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding([28, 32])
            .align_x(Alignment::Center)
            .into()
    }

    fn wizard_looks<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let looks: Vec<&Preset> = self
            .profiles
            .presets()
            .iter()
            .filter(|preset| {
                preset.id != crate::desktop::profiles::CLEAN && !preset.profile.appearance_only
            })
            .collect();
        // Three tiles a row; the miniature fills its tile at 16:9.
        // Two rows always fit the window's height too, without scrolling.
        let by_width = ((self.wizard_inner_width() - 24.) / 3. - 14.) * 9. / 16.;
        let by_height = (self.window.height - 290.) / 2. - 38.;
        let thumb_height = by_width.min(by_height).floor().max(80.);
        let tile = |preset: &'a Preset| -> Element<'a, Message> {
            let selected = wizard.look == Some(preset.id);
            button::custom(
                column::with_children(vec![
                    self.preset_thumbnail(preset, thumb_height),
                    row::with_children(vec![
                        container(words(preset.profile.name.clone(), 13., INK))
                            .width(Length::Fill)
                            .into(),
                        words(if selected { "✓" } else { "" }, 13., LAVENDER),
                    ])
                    .align_y(Alignment::Center)
                    .into(),
                ])
                .spacing(6),
            )
            .id(cosmic::widget::Id::new(format!("wizard-look-{}", preset.id)))
            .class(button_style(
                if selected {
                    Color::from_rgba(0.60, 0.53, 0.98, 0.16)
                } else {
                    TILE
                },
                Color::from_rgba(1., 1., 1., 0.09),
                if selected { LAVENDER } else { EDGE },
                12.,
            ))
            .padding(7)
            .width(Length::Fill)
            .on_press(go(Step::Look(preset.id)))
            .into()
        };
        let mut rows = Vec::new();
        for chunk in looks.chunks(3) {
            let mut cells: Vec<Element<'a, Message>> = chunk.iter().map(|p| tile(p)).collect();
            while cells.len() < 3 {
                cells.push(cosmic::iced::widget::space().width(Length::Fill).into());
            }
            rows.push(row::with_children(cells).spacing(12).into());
        }
        column::with_children(rows).spacing(12).into()
    }

    fn wizard_bars<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let bars = wizard.bars;
        let column_width = (self.wizard_inner_width() - 18.) / 2.;
        let tile_width = (column_width - 8. * 2.) / 3.;
        let draw = tile_width - 24.;
        let line = |name: &'static str, choices: Vec<Element<'a, Message>>| -> Element<'a, Message> {
            column::with_children(vec![
                words(name, 12., SOFT),
                row::with_children(choices).spacing(8).into(),
            ])
            .spacing(4)
            .into()
        };
        let shapes = |current: Shape, part: &'static str, message: fn(Shape) -> Step| {
            Shape::ALL
                .into_iter()
                .map(|candidate| {
                    choice(
                        shape(draw * 0.8, 16., corner(candidate, 16.), LAVENDER),
                        candidate.title(),
                        format!("wizard-{part}-{}", candidate.title()).to_lowercase(),
                        current == candidate,
                        Some(go(message(candidate))),
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut panel = vec![
            words("Panel", 15., INK),
            line(
                "Style",
                vec![
                    choice(
                        panel_drawing(false, bars.panel_shape, draw, 12.),
                        "Bar",
                        "wizard-panel-bar".into(),
                        !bars.islands,
                        Some(go(Step::Islands(false))),
                    ),
                    choice(
                        panel_drawing(true, bars.panel_shape, draw, 12.),
                        "Islands",
                        "wizard-panel-islands".into(),
                        bars.islands,
                        wizard.islands_available.then(|| go(Step::Islands(true))),
                    ),
                ],
            ),
            line("Shape", shapes(bars.panel_shape, "panel", Step::PanelShape)),
        ];
        if !wizard.islands_available {
            panel.push(words(
                "Islands needs the Halcyon panel component, which isn’t running here.",
                12.,
                SOFT,
            ));
        }
        let edge = || shape(46., 2., 1., Color::from_rgba(1., 1., 1., 0.35));
        let position = |floating: bool| -> Element<'a, Message> {
            column::with_children(vec![
                dock_drawing(bars.dock_shape, 5.),
                cosmic::iced::widget::space().height(if floating { 5. } else { 0. }).into(),
                edge(),
            ])
            .align_x(Alignment::Center)
            .into()
        };
        let dock = vec![
            words("Dock", 15., INK),
            line("Shape", shapes(bars.dock_shape, "dock", Step::DockShape)),
            line(
                "Behavior",
                vec![
                    choice(
                        position(true),
                        "Floating",
                        "wizard-dock-floating".into(),
                        bars.floating,
                        Some(go(Step::Floating(true))),
                    ),
                    choice(
                        position(false),
                        "Attached",
                        "wizard-dock-attached".into(),
                        !bars.floating,
                        Some(go(Step::Floating(false))),
                    ),
                    choice(
                        dock_drawing(bars.dock_shape, 5.),
                        "Shown",
                        "wizard-dock-shown".into(),
                        !bars.hides,
                        Some(go(Step::Hides(false))),
                    ),
                    choice(
                        column::with_children(vec![words("↓", 13., SOFT), edge()])
                            .align_x(Alignment::Center)
                            .into(),
                        "Hides",
                        "wizard-dock-hides".into(),
                        bars.hides,
                        Some(go(Step::Hides(true))),
                    ),
                ],
            ),
        ];
        column::with_children(vec![
            self.wizard_bars_preview(wizard),
            row::with_children(vec![
                column::with_children(panel).spacing(10).width(column_width).into(),
                column::with_children(dock).spacing(10).width(column_width).into(),
            ])
            .spacing(18)
            .into(),
        ])
        .spacing(16)
        .into()
    }

    /// The chosen look's wallpaper, full width, with the panel on top and
    /// the dock below as they will look.
    fn wizard_bars_preview<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let bars = wizard.bars;
        let w = self.wizard_inner_width();
        // What the window height leaves above the two choice columns.
        let h = (self.window.height - 520.).min(w * 0.24).clamp(80., 190.).floor();
        let background: Element<'a, Message> = match band(wizard.look, w / h) {
            Some(handle) => cosmic::widget::image(handle)
                .width(w)
                .height(h)
                .content_fit(cosmic::iced::ContentFit::Fill)
                .into(),
            None => shape(w, h, 0., Color::from_rgb(0.08, 0.08, 0.2)),
        };
        let panel_gap = if bars.islands { 6. } else { 0. };
        let overlay = column::with_children(vec![
            container(panel_drawing(bars.islands, bars.panel_shape, w - 2. * panel_gap - 16., 18.))
                .width(Length::Fill)
                .padding([panel_gap, 8. + panel_gap])
                .align_x(Alignment::Center)
                .into(),
            cosmic::iced::widget::space().height(Length::Fill).into(),
            container(if bars.hides {
                shape(60., 4., 2., Color::from_rgba(1., 1., 1., 0.5))
            } else {
                dock_drawing(bars.dock_shape, 9.)
            })
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding([0., 0., if bars.floating { 8. } else { 0. }, 0.])
            .into(),
        ])
        .width(w)
        .height(h);
        fill_box(
            stack(vec![background, overlay.into()])
                .width(w)
                .height(h)
                .clip(true)
                .into(),
            Color::TRANSPARENT,
            EDGE,
            12.,
        )
        .clip(true)
        .into()
    }

    fn wizard_weather(&self) -> Element<'_, Message> {
        let state = &self.city_search;
        let mut parts = Vec::new();
        if let Some(location) = &self.config.location {
            parts.push(words(format!("Current: {}", location.name), 14., INK));
        }
        let search = text_input("City name", &state.query)
            .id(cosmic::widget::Id::new("wizard-city"))
            .on_input(Message::CityInput)
            .on_submit(|_| Message::SearchCity)
            .width(Length::Fill);
        parts.push(search.into());
        if state.searching || state.deadline.is_some() {
            parts.push(words("Searching cities…", 13., SOFT));
        } else if let Some(error) = &state.error {
            parts.push(words(error.clone(), 13., DANGER));
        } else if !self.cities.is_empty() {
            for (index, city) in self.cities.iter().enumerate() {
                let selected = self.config.location.as_ref().is_some_and(|location| {
                    location.latitude == city.latitude && location.longitude == city.longitude
                });
                parts.push(
                    button::custom(
                        row::with_children(vec![
                            column::with_children(vec![
                                words(city.name.clone(), 14., INK),
                                words(city.region(), 12., SOFT),
                            ])
                            .width(Length::Fill)
                            .into(),
                            words(if selected { "✓" } else { "Choose" }, 13., LAVENDER),
                        ])
                        .align_y(Alignment::Center),
                    )
                    .id(cosmic::widget::Id::new(format!("wizard-city-{index}")))
                    .class(button_style(
                        TILE,
                        Color::from_rgba(1., 1., 1., 0.09),
                        if selected { LAVENDER } else { EDGE },
                        10.,
                    ))
                    .padding([8, 12])
                    .width(Length::Fill)
                    .on_press(Message::ChooseCity(index))
                    .into(),
                );
            }
        } else if state.searched {
            parts.push(words("No cities found. Try a nearby town.", 13., SOFT));
        }
        column::with_children(parts).spacing(10).into()
    }

    fn wizard_ready<'a>(&'a self, wizard: &'a Wizard) -> Element<'a, Message> {
        let bars = wizard.bars;
        let look = wizard
            .look
            .and_then(|id| {
                self.profiles
                    .presets()
                    .iter()
                    .find(|preset| preset.id == id)
                    .map(|preset| preset.profile.name.clone())
            })
            .unwrap_or_else(|| "Unchanged".into());
        let panel = format!(
            "{} {}, {} dock",
            bars.panel_shape.title(),
            if bars.islands { "islands" } else { "bar" },
            bars.dock_shape.title().to_lowercase()
        );
        let city = self
            .config
            .location
            .as_ref()
            .map_or_else(|| "Not set".to_string(), |location| location.name.clone());
        let summary = |name: &'static str, value: String, target: Screen| -> Element<'a, Message> {
            fill_box(
                row::with_children(vec![
                    container(words(name, 14., INK)).width(Length::Fill).into(),
                    words(value, 14., SOFT),
                    button::custom(words("Change", 13., LAVENDER))
                        .class(button_style(
                            Color::TRANSPARENT,
                            Color::from_rgba(1., 1., 1., 0.06),
                            Color::TRANSPARENT,
                            8.,
                        ))
                        .padding([4, 8])
                        .on_press(go(Step::Show(target)))
                        .into(),
                ])
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
                TILE,
                EDGE,
                10.,
            )
            .padding([8, 14])
            .width(Length::Fill)
            .into()
        };
        column::with_children(vec![
            summary("Look", look, Screen::Look),
            summary("Dock & panel", panel, Screen::Bars),
            summary("Weather", city, Screen::Weather),
            fill_box(
                row::with_children(vec![
                    container(words("Start at sign-in", 14., INK))
                        .width(Length::Fill)
                        .into(),
                    toggler(wizard.startup)
                        .on_toggle(|on| go(Step::Startup(on)))
                        .into(),
                ])
                .align_y(Alignment::Center)
                .into(),
                TILE,
                EDGE,
                10.,
            )
            .padding([10, 14])
            .width(Length::Fill)
            .into(),
        ])
        .spacing(8)
        .into()
    }
}
