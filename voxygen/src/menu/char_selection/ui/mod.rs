use crate::{
    i18n::{i18n_asset_key, Localization},
    render::Renderer,
    ui::{
        self,
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            style,
            widget::{
                mouse_detector, AspectRatioContainer, BackgroundContainer, Image, MouseDetector,
                Overlay, Padding,
            },
            Element, IcedRenderer, IcedUi as Ui,
        },
        img_ids::{ImageGraphic, VoxelGraphic},
    },
    window, GlobalState,
};
use client::Client;
use common::{
    character::{CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp::{self, humanoid},
    LoadoutBuilder,
};
//ImageFrame, Tooltip,
use crate::settings::Settings;
use common::assets::load_expect;
//use std::time::Duration;
//use ui::ice::widget;
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, HorizontalAlignment, Length,
    Row, Scrollable, Space, Text, TextInput,
};
use vek::Rgba;

pub const TEXT_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 1.0);
pub const DISABLED_TEXT_COLOR: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2);
const FILL_FRAC_ONE: f32 = 0.77;
const FILL_FRAC_TWO: f32 = 0.60;

const STARTER_HAMMER: &str = "common.items.weapons.hammer.starter_hammer";
const STARTER_BOW: &str = "common.items.weapons.bow.starter_bow";
const STARTER_AXE: &str = "common.items.weapons.axe.starter_axe";
const STARTER_STAFF: &str = "common.items.weapons.staff.starter_staff";
const STARTER_SWORD: &str = "common.items.weapons.sword.starter_sword";
const STARTER_DAGGER: &str = "common.items.weapons.dagger.starter_dagger";

// TODO: look into what was using this in old ui
const UI_MAIN: iced::Color = iced::Color::from_rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue

image_ids_ice! {
    struct Imgs {
        <VoxelGraphic>
        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        <ImageGraphic>
        gray_corner: "voxygen.element.frames.gray.corner",
        gray_edge: "voxygen.element.frames.gray.edge",

        selection: "voxygen.element.frames.selection",
        selection_hover: "voxygen.element.frames.selection_hover",
        selection_press: "voxygen.element.frames.selection_press",

        delete_button: "voxygen.element.buttons.x_red",
        delete_button_hover: "voxygen.element.buttons.x_red_hover",
        delete_button_press: "voxygen.element.buttons.x_red_press",

        name_input: "voxygen.element.misc_bg.textbox",

        // Tool Icons
        daggers: "voxygen.element.icons.daggers",
        sword: "voxygen.element.icons.sword",
        axe: "voxygen.element.icons.axe",
        hammer: "voxygen.element.icons.hammer",
        bow: "voxygen.element.icons.bow",
        staff: "voxygen.element.icons.staff",

        // Species Icons
        male: "voxygen.element.icons.male",
        female: "voxygen.element.icons.female",
        human_m: "voxygen.element.icons.human_m",
        human_f: "voxygen.element.icons.human_f",
        orc_m: "voxygen.element.icons.orc_m",
        orc_f: "voxygen.element.icons.orc_f",
        dwarf_m: "voxygen.element.icons.dwarf_m",
        dwarf_f: "voxygen.element.icons.dwarf_f",
        undead_m: "voxygen.element.icons.ud_m",
        undead_f: "voxygen.element.icons.ud_f",
        elf_m: "voxygen.element.icons.elf_m",
        elf_f: "voxygen.element.icons.elf_f",
        danari_m: "voxygen.element.icons.danari_m",
        danari_f: "voxygen.element.icons.danari_f",
        // Icon Borders
        icon_border: "voxygen.element.buttons.border",
        icon_border_mo: "voxygen.element.buttons.border_mo",
        icon_border_press: "voxygen.element.buttons.border_press",
        icon_border_pressed: "voxygen.element.buttons.border_pressed",

        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",
    }
}

// TODO: do rotation in widget renderer
/*rotation_image_ids! {
    pub struct ImgsRot {
        <VoxelGraphic>

        // Tooltip Test
        tt_side: "voxygen/element/frames/tt_test_edge",
        tt_corner: "voxygen/element/frames/tt_test_corner_tr",
    }
}*/

pub enum Event {
    Logout,
    Play(i32),
    AddCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(i32),
}

enum Mode {
    Select {
        info_content: Option<InfoContent>,
        // Index of selected character
        selected: Option<usize>,

        characters_scroll: scrollable::State,
        character_buttons: Vec<button::State>,
        new_character_button: button::State,
        logout_button: button::State,
        enter_world_button: button::State,
        change_server_button: button::State,
        yes_button: button::State,
        no_button: button::State,
    },
    Create {
        name: String, // TODO: default to username
        body: humanoid::Body,
        loadout: comp::Loadout,
        // TODO: does this need to be an option, never seems to be none
        tool: Option<&'static str>,

        body_type_buttons: [button::State; 2],
        species_buttons: [button::State; 6],
        tool_buttons: [button::State; 6],
        scroll: scrollable::State,
        name_input: text_input::State,
        back_button: button::State,
        create_button: button::State,
    },
}

impl Mode {
    pub fn select() -> Self {
        Self::Select {
            info_content: None,
            selected: None,
            characters_scroll: Default::default(),
            character_buttons: Vec::new(),
            new_character_button: Default::default(),
            logout_button: Default::default(),
            enter_world_button: Default::default(),
            change_server_button: Default::default(),
            yes_button: Default::default(),
            no_button: Default::default(),
        }
    }

    pub fn create(name: String) -> Self {
        let tool = Some(STARTER_SWORD);
        let loadout = LoadoutBuilder::new()
            .defaults()
            .active_item(LoadoutBuilder::default_item_config_from_str(tool))
            .build();

        Self::Create {
            name,
            body: humanoid::Body::random(),
            loadout,
            tool,

            body_type_buttons: Default::default(),
            species_buttons: Default::default(),
            tool_buttons: Default::default(),
            scroll: Default::default(),
            name_input: Default::default(),
            back_button: Default::default(),
            create_button: Default::default(),
        }
    }
}

#[derive(PartialEq)]
enum InfoContent {
    Deletion(usize),
    LoadingCharacters,
    CreatingCharacter,
    DeletingCharacter,
    CharacterError,
}

/*
impl InfoContent {
    pub fn has_content(&self, character_list_loading: &bool) -> bool {
        match self {
            Self::None => false,
            Self::CreatingCharacter | Self::DeletingCharacter | Self::LoadingCharacters => {
                *character_list_loading
            },
            _ => true,
        }
    }
}
*/

struct Controls {
    fonts: Fonts,
    imgs: Imgs,
    i18n: std::sync::Arc<Localization>,
    // Voxygen version
    version: String,
    // Alpha disclaimer
    alpha: String,

    // Zone for rotating the character with the mouse
    mouse_detector: mouse_detector::State,
    // enter: bool,
    mode: Mode,
}

#[derive(Clone)]
enum Message {
    Back,
    Logout,
    EnterWorld,
    Select(usize),
    Delete(usize),
    ChangeServer,
    NewCharacter,
    CreateCharacter,
    Name(String),
    BodyType(humanoid::BodyType),
    Species(humanoid::Species),
    Tool(&'static str),
    CancelDeletion,
    ConfirmDeletion,
}

impl Controls {
    fn new(fonts: Fonts, imgs: Imgs, i18n: std::sync::Arc<Localization>) -> Self {
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );
        let alpha = format!("Veloren Pre-Alpha {}", env!("CARGO_PKG_VERSION"),);

        Self {
            fonts,
            imgs,
            i18n,
            version,
            alpha,

            mouse_detector: Default::default(),
            mode: Mode::select(),
        }
    }

    fn view(&mut self, settings: &Settings, client: &Client) -> Element<Message> {
        // TODO: use font scale thing for text size (use on button size for buttons with
        // text) TODO: if enter key pressed and character is selected then enter
        // the world TODO: tooltip widget

        let imgs = &self.imgs;
        let fonts = &self.fonts;
        let i18n = &self.i18n;

        let button_style = style::button::Style::new(imgs.button)
            .hover_image(imgs.button_hover)
            .press_image(imgs.button_press)
            .text_color(TEXT_COLOR)
            .disabled_text_color(DISABLED_TEXT_COLOR);

        let version = iced::Text::new(&self.version)
            .size(self.fonts.cyri.scale(15))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Right);

        let alpha = iced::Text::new(&self.alpha)
            .size(self.fonts.cyri.scale(12))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Center);

        let top_text = Row::with_children(vec![
            Space::new(Length::Fill, Length::Shrink).into(),
            alpha.into(),
            version.into(),
        ])
        .width(Length::Fill);

        let content = match &mut self.mode {
            Mode::Select {
                info_content,
                selected,
                ref mut characters_scroll,
                ref mut character_buttons,
                ref mut new_character_button,
                ref mut logout_button,
                ref mut enter_world_button,
                ref mut change_server_button,
                ref mut yes_button,
                ref mut no_button,
            } => {
                let server = Container::new(
                    Column::with_children(vec![
                        Text::new(&client.server_info.name)
                            .size(fonts.cyri.scale(25))
                            .into(),
                        Container::new(neat_button(
                            change_server_button,
                            i18n.get("char_selection.change_server"),
                            FILL_FRAC_TWO,
                            button_style,
                            Some(Message::ChangeServer),
                        ))
                        .height(Length::Units(35))
                        .into(),
                    ])
                    .spacing(5)
                    .align_items(Align::Center),
                )
                .style(style::container::Style::color_with_image_border(
                    Rgba::new(0, 0, 0, 217),
                    imgs.gray_corner,
                    imgs.gray_edge,
                ))
                .padding(12)
                .center_x()
                .width(Length::Fill);

                let characters = {
                    let characters = &client.character_list.characters;
                    let num = characters.len();
                    // Ensure we have enough button states
                    character_buttons.resize_with(num * 2, Default::default);

                    let mut characters = characters
                        .iter()
                        .zip(character_buttons.chunks_exact_mut(2))
                        .map(|(character, buttons)| {
                            let mut buttons = buttons.iter_mut();
                            (
                                character,
                                (buttons.next().unwrap(), buttons.next().unwrap()),
                            )
                        })
                        .enumerate()
                        .map(|(i, (character, (select_button, delete_button)))| {
                            Overlay::new(
                                Button::new(
                                    select_button,
                                    Space::new(Length::Units(16), Length::Units(16)),
                                )
                                .style(
                                    style::button::Style::new(imgs.delete_button)
                                        .hover_image(imgs.delete_button_hover)
                                        .press_image(imgs.delete_button_press),
                                )
                                .on_press(Message::Delete(i)),
                                AspectRatioContainer::new(
                                    Button::new(
                                        delete_button,
                                        Column::with_children(vec![
                                            Text::new("Hi").into(),
                                            Text::new("Hi").into(),
                                            Text::new("Hi").into(),
                                        ]),
                                    )
                                    .style(
                                        style::button::Style::new(imgs.selection)
                                            .hover_image(imgs.selection_hover)
                                            .press_image(imgs.selection_press),
                                    )
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .on_press(Message::Select(i)),
                                )
                                .ratio_of_image(imgs.selection),
                            )
                            .padding(12)
                            .align_x(Align::End)
                            .into()
                        })
                        .collect::<Vec<_>>();

                    // Add create new character button
                    let color = if num >= MAX_CHARACTERS_PER_PLAYER {
                        (97, 97, 25)
                    } else {
                        (97, 255, 18)
                    };
                    characters.push(
                        AspectRatioContainer::new({
                            let button = Button::new(
                                new_character_button,
                                Container::new(Text::new(
                                    i18n.get("char_selection.create_new_character"),
                                ))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                            )
                            .style(
                                style::button::Style::new(imgs.selection)
                                    .hover_image(imgs.selection_hover)
                                    .press_image(imgs.selection_press)
                                    .image_color(Rgba::new(color.0, color.1, color.2, 255))
                                    .text_color(iced::Color::from_rgb8(color.0, color.1, color.2))
                                    .disabled_text_color(iced::Color::from_rgb8(
                                        color.0, color.1, color.2,
                                    )),
                            )
                            .width(Length::Fill)
                            .height(Length::Fill);
                            if num < MAX_CHARACTERS_PER_PLAYER {
                                button.on_press(Message::NewCharacter)
                            } else {
                                button
                            }
                        })
                        .ratio_of_image(imgs.selection)
                        .into(),
                    );
                    characters
                };

                // TODO: could replace column with scrollable completely if it had a with
                // children method
                let characters = Container::new(
                    Scrollable::new(characters_scroll)
                        .push(Column::with_children(characters).spacing(4)),
                )
                .style(style::container::Style::color_with_image_border(
                    Rgba::new(0, 0, 0, 217),
                    imgs.gray_corner,
                    imgs.gray_edge,
                ))
                .padding(9)
                .width(Length::Fill)
                .height(Length::Fill);

                let right_column = Column::with_children(vec![server.into(), characters.into()])
                    .padding(15)
                    .spacing(10)
                    .width(Length::Units(360)) // TODO: see if we can get iced to work with settings below
                    //.max_width(360)
                    //.width(Length::Fill)
                    .height(Length::Fill);

                let top = Row::with_children(vec![
                    right_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(15)
                .width(Length::Fill)
                .height(Length::Fill);

                let logout = neat_button(
                    logout_button,
                    i18n.get("char_selection.logout"),
                    FILL_FRAC_ONE,
                    button_style,
                    Some(Message::Logout),
                );

                let enter_world = neat_button(
                    enter_world_button,
                    i18n.get("char_selection.enter_world"),
                    FILL_FRAC_ONE,
                    button_style,
                    selected.map(|_| Message::EnterWorld),
                );

                let bottom = Row::with_children(vec![
                    Container::new(logout)
                        .width(Length::Fill)
                        .height(Length::Units(40))
                        .into(),
                    Container::new(enter_world)
                        .width(Length::Fill)
                        .height(Length::Units(60))
                        .center_x()
                        .into(),
                    Space::new(Length::Fill, Length::Shrink).into(),
                ])
                .align_items(Align::End);

                let content = Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .height(Length::Fill);

                // Overlay delete prompt
                if let Some(info_content) = info_content {
                    let over: Element<_> = match info_content {
                        InfoContent::Deletion(_) => Container::new(
                            Column::with_children(vec![
                                Text::new(self.i18n.get("char_selection.delete_permanently"))
                                    .size(fonts.cyri.scale(24))
                                    .into(),
                                Row::with_children(vec![
                                    neat_button(
                                        no_button,
                                        i18n.get("common.no"),
                                        FILL_FRAC_ONE,
                                        button_style,
                                        Some(Message::CancelDeletion),
                                    ),
                                    neat_button(
                                        yes_button,
                                        i18n.get("common.yes"),
                                        FILL_FRAC_ONE,
                                        button_style,
                                        Some(Message::ConfirmDeletion),
                                    ),
                                ])
                                .height(Length::Units(28))
                                .spacing(30)
                                .into(),
                            ])
                            .align_items(Align::Center)
                            .spacing(10),
                        )
                        .style(
                            style::container::Style::color_with_double_cornerless_border(
                                (0, 0, 0, 200).into(),
                                (3, 4, 4, 255).into(),
                                (28, 28, 22, 255).into(),
                            ),
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .max_width(400)
                        .max_height(130)
                        .padding(16)
                        .center_x()
                        .center_y()
                        .into(),
                        // TODO
                        _ => Space::new(Length::Shrink, Length::Shrink).into(),
                    };

                    Overlay::new(over, content)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .into()
                } else {
                    content.into()
                }
            },
            Mode::Create {
                name,
                body,
                loadout,
                tool,
                ref mut scroll,
                ref mut body_type_buttons,
                ref mut species_buttons,
                ref mut tool_buttons,
                ref mut name_input,
                ref mut back_button,
                ref mut create_button,
            } => {
                let unselected_style = style::button::Style::new(imgs.icon_border)
                    .hover_image(imgs.icon_border_mo)
                    .press_image(imgs.icon_border_press);

                let selected_style = style::button::Style::new(imgs.icon_border_pressed)
                    .hover_image(imgs.icon_border_mo)
                    .press_image(imgs.icon_border_press);

                let icon_button = |button, selected, msg, img| {
                    Container::new(
                        Button::<_, IcedRenderer>::new(
                            button,
                            Space::new(Length::Units(70), Length::Units(70)),
                        )
                        .style(if selected {
                            selected_style
                        } else {
                            unselected_style
                        })
                        .on_press(msg),
                    )
                    .style(style::container::Style::image(img))
                };

                let [ref mut male_button, ref mut female_button] = body_type_buttons;
                let body_type = Row::with_children(vec![
                    icon_button(
                        male_button,
                        matches!(body.body_type, humanoid::BodyType::Male),
                        Message::BodyType(humanoid::BodyType::Male),
                        imgs.male,
                    )
                    .into(),
                    icon_button(
                        female_button,
                        matches!(body.body_type, humanoid::BodyType::Female),
                        Message::BodyType(humanoid::BodyType::Female),
                        imgs.female,
                    )
                    .into(),
                ])
                .spacing(1);

                let (human_icon, orc_icon, dwarf_icon, elf_icon, undead_icon, danari_icon) =
                    match body.body_type {
                        humanoid::BodyType::Male => (
                            self.imgs.human_m,
                            self.imgs.orc_m,
                            self.imgs.dwarf_m,
                            self.imgs.elf_m,
                            self.imgs.undead_m,
                            self.imgs.danari_m,
                        ),
                        humanoid::BodyType::Female => (
                            self.imgs.human_f,
                            self.imgs.orc_f,
                            self.imgs.dwarf_f,
                            self.imgs.elf_f,
                            self.imgs.undead_f,
                            self.imgs.danari_f,
                        ),
                    };

                // TODO: tooltips
                let [ref mut human_button, ref mut orc_button, ref mut dwarf_button, ref mut elf_button, ref mut undead_button, ref mut danari_button] =
                    species_buttons;
                let species = Column::with_children(vec![
                    Row::with_children(vec![
                        icon_button(
                            human_button,
                            matches!(body.species, humanoid::Species::Human),
                            Message::Species(humanoid::Species::Human),
                            human_icon,
                        )
                        .into(),
                        icon_button(
                            orc_button,
                            matches!(body.species, humanoid::Species::Orc),
                            Message::Species(humanoid::Species::Orc),
                            orc_icon,
                        )
                        .into(),
                        icon_button(
                            dwarf_button,
                            matches!(body.species, humanoid::Species::Dwarf),
                            Message::Species(humanoid::Species::Dwarf),
                            dwarf_icon,
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                    Row::with_children(vec![
                        icon_button(
                            elf_button,
                            matches!(body.species, humanoid::Species::Elf),
                            Message::Species(humanoid::Species::Elf),
                            elf_icon,
                        )
                        .into(),
                        icon_button(
                            undead_button,
                            matches!(body.species, humanoid::Species::Undead),
                            Message::Species(humanoid::Species::Undead),
                            undead_icon,
                        )
                        .into(),
                        icon_button(
                            danari_button,
                            matches!(body.species, humanoid::Species::Danari),
                            Message::Species(humanoid::Species::Danari),
                            danari_icon,
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                ])
                .spacing(1);

                let [ref mut sword_button, ref mut daggers_button, ref mut axe_button, ref mut hammer_button, ref mut bow_button, ref mut staff_button] =
                    tool_buttons;
                let tool = Column::with_children(vec![
                    Row::with_children(vec![
                        icon_button(
                            sword_button,
                            matches!(tool, Some(STARTER_SWORD)),
                            Message::Tool(STARTER_SWORD),
                            imgs.sword,
                        )
                        .into(),
                        icon_button(
                            daggers_button,
                            matches!(tool, Some(STARTER_DAGGER)),
                            // TODO: pass none
                            Message::Tool(STARTER_DAGGER),
                            imgs.daggers,
                        )
                        .into(),
                        icon_button(
                            axe_button,
                            matches!(tool, Some(STARTER_AXE)),
                            Message::Tool(STARTER_AXE),
                            imgs.axe,
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                    Row::with_children(vec![
                        icon_button(
                            hammer_button,
                            matches!(tool, Some(STARTER_HAMMER)),
                            Message::Tool(STARTER_HAMMER),
                            imgs.hammer,
                        )
                        .into(),
                        icon_button(
                            bow_button,
                            matches!(tool, Some(STARTER_BOW)),
                            Message::Tool(STARTER_BOW),
                            imgs.bow,
                        )
                        .into(),
                        icon_button(
                            staff_button,
                            matches!(tool, Some(STARTER_STAFF)),
                            Message::Tool(STARTER_STAFF),
                            imgs.staff,
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                ])
                .spacing(1);

                let column_content = vec![body_type.into(), species.into(), tool.into()];

                let right_column = Container::new(
                    Column::with_children(vec![
                        Text::new(i18n.get("char_selection.character_creation"))
                            .size(fonts.cyri.scale(26))
                            .into(),
                        Scrollable::new(scroll)
                            .push(
                                Column::with_children(column_content)
                                    .align_items(Align::Center)
                                    .spacing(16),
                            )
                            .into(),
                    ])
                    .spacing(20)
                    .padding(10)
                    .width(Length::Fill)
                    .align_items(Align::Center),
                )
                .style(style::container::Style::color_with_image_border(
                    Rgba::new(0, 0, 0, 217),
                    imgs.gray_corner,
                    imgs.gray_edge,
                ))
                .padding(10)
                .width(Length::Units(360)) // TODO: see if we can get iced to work with settings below
                //.max_width(360)
                //.width(Length::Fill)
                .height(Length::Fill);

                let top = Row::with_children(vec![
                    right_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(15)
                .width(Length::Fill)
                .height(Length::Fill);

                let back = neat_button(
                    back_button,
                    i18n.get("common.back"),
                    FILL_FRAC_ONE,
                    button_style,
                    Some(Message::Back),
                );

                let name_input = BackgroundContainer::new(
                    Image::new(imgs.name_input)
                        .height(Length::Units(40))
                        .fix_aspect_ratio(),
                    TextInput::new(name_input, "Character Name", &name, Message::Name)
                        .size(25)
                        .on_submit(Message::CreateCharacter),
                )
                .padding(Padding::new().horizontal(7).top(5));

                let create = neat_button(
                    create_button,
                    i18n.get("common.create"),
                    FILL_FRAC_ONE,
                    button_style,
                    (!name.is_empty()).then_some(Message::CreateCharacter),
                );

                let bottom = Row::with_children(vec![
                    Container::new(back)
                        .width(Length::Fill)
                        .height(Length::Units(40))
                        .into(),
                    Container::new(name_input)
                        .width(Length::Fill)
                        .center_x()
                        .into(),
                    Container::new(create)
                        .width(Length::Fill)
                        .height(Length::Units(40))
                        .align_x(Align::End)
                        .into(),
                ])
                .align_items(Align::End);

                Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            },
        };

        Container::new(
            Column::with_children(vec![top_text.into(), content])
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(3)
        .into()
    }

    fn update(&mut self, message: Message, events: &mut Vec<Event>, characters: &[CharacterItem]) {
        match message {
            Message::Back => {
                if matches!(&self.mode, Mode::Create { .. }) {
                    self.mode = Mode::select();
                }
            },
            Message::Logout => {
                events.push(Event::Logout);
            },
            Message::EnterWorld => {
                if let Mode::Select {
                    selected: Some(selected),
                    ..
                } = &self.mode
                {
                    // TODO: eliminate option in character id
                    if let Some(id) = characters.get(*selected).and_then(|i| i.character.id) {
                        events.push(Event::Play(id));
                    }
                }
            },
            Message::Select(idx) => {
                if let Mode::Select { selected, .. } = &mut self.mode {
                    *selected = Some(idx);
                }
            },
            Message::Delete(idx) => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    *info_content = Some(InfoContent::Deletion(idx));
                }
            },
            Message::ChangeServer => {
                events.push(Event::Logout);
            },
            Message::NewCharacter => {
                if matches!(&self.mode, Mode::Select { .. }) {
                    self.mode = Mode::create(String::new());
                }
            },
            Message::CreateCharacter => {
                if let Mode::Create {
                    name, body, tool, ..
                } = &self.mode
                {
                    events.push(Event::AddCharacter {
                        alias: name.clone(),
                        tool: tool.map(String::from),
                        body: comp::Body::Humanoid(*body),
                    });
                    self.mode = Mode::select();
                }
            },
            Message::Name(value) => {
                if let Mode::Create { name, .. } = &mut self.mode {
                    *name = value;
                }
            },
            Message::BodyType(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.body_type = value;
                    body.validate();
                }
            },
            Message::Species(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.species = value;
                    body.validate();
                }
            },
            Message::Tool(value) => {
                if let Mode::Create { tool, loadout, .. } = &mut self.mode {
                    *tool = Some(value);
                    loadout.active_item = LoadoutBuilder::default_item_config_from_str(*tool);
                }
            },
            Message::ConfirmDeletion => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    if let Some(InfoContent::Deletion(idx)) = info_content {
                        if let Some(id) = characters.get(*idx).and_then(|i| i.character.id) {
                            events.push(Event::DeleteCharacter(id));
                        }
                        *info_content = None;
                    }
                }
            },
            Message::CancelDeletion => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    if let Some(InfoContent::Deletion(idx)) = info_content {
                        *info_content = None;
                    }
                }
            },
        }
    }

    /// Get the character to display
    pub fn display_body_loadout<'a>(
        &'a self,
        characters: &'a [CharacterItem],
    ) -> Option<(comp::Body, &'a comp::Loadout)> {
        match &self.mode {
            Mode::Select { selected, .. } => selected
                .and_then(|idx| characters.get(idx))
                .map(|i| (i.body, &i.loadout)),
            Mode::Create { loadout, body, .. } => Some((comp::Body::Humanoid(*body), loadout)),
        }
    }
}

pub struct CharSelectionUi {
    ui: Ui,
    controls: Controls,
}

impl CharSelectionUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        // Load language
        let i18n = load_expect::<Localization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));

        // TODO: don't add default font twice
        let font = {
            use std::io::Read;
            let mut buf = Vec::new();
            common::assets::load_file("voxygen.font.haxrcorp_4089_cyrillic_altgr_extended", &[
                "ttf",
            ])
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
            ui::ice::Font::try_from_vec(buf).unwrap()
        };

        let mut ui = Ui::new(&mut global_state.window, font).unwrap();

        let fonts = Fonts::load(&i18n.fonts, &mut ui).expect("Impossible to load fonts");

        let controls = Controls::new(
            fonts,
            Imgs::load(&mut ui).expect("Failed to load images"),
            i18n,
        );

        Self { ui, controls }
    }

    pub fn display_body_loadout<'a>(
        &'a self,
        characters: &'a [CharacterItem],
    ) -> Option<(comp::Body, &'a comp::Loadout)> {
        self.controls.display_body_loadout(characters)
    }

    pub fn handle_event(&mut self, event: window::Event) -> bool {
        match event {
            window::Event::IcedUi(event) => {
                self.ui.handle_event(event);
                true
            },
            window::Event::MouseButton(_, window::PressState::Pressed) => {
                !self.controls.mouse_detector.mouse_over()
            },
            _ => false,
        }
    }

    // TODO: do we need whole client here or just character list
    pub fn maintain(&mut self, global_state: &mut GlobalState, client: &mut Client) -> Vec<Event> {
        let mut events = Vec::new();

        let (messages, _) = self.ui.maintain(
            self.controls.view(&global_state.settings, &client),
            global_state.window.renderer_mut(),
        );

        messages.into_iter().for_each(|message| {
            self.controls
                .update(message, &mut events, &client.character_list.characters)
        });

        events
    }

    // TODO: do we need globals
    pub fn render(&self, renderer: &mut Renderer) { self.ui.render(renderer); }
}
